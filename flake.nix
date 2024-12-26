{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  nixConfig = {
    extra-substituters = [
      "https://cache.nixos.org"
      "https://nix-community.cachix.org"
      "https://toyvo.cachix.org"
    ];
    extra-trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "toyvo.cachix.org-1:s++CG1te6YaS9mjICre0Ybbya2o/S9fZIyDNGiD4UXs="
    ];
    allow-import-from-derivation = true;
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      flake-parts,
      devshell,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      imports = [
        devshell.flakeModule
        flake-parts.flakeModules.easyOverlay
      ];

      flake = {
        nixosModules.discord_bot =
          {
            pkgs,
            lib,
            config,
            ...
          }:
          let
            cfg = config.services.discord_bot;
          in
          {
            options.services.discord_bot = {
              enable = lib.mkEnableOption "enable discord bot";
              env_file = lib.mkOption {
                type = lib.types.path;
                description = ''
                  Path to the environment file, to be piped through xargs, must include the following variables:
                  DISCORD_CLIENT_ID
                  DISCORD_CLIENT_SECRET
                  DISCORD_PUBLIC_KEY
                  DISCORD_TOKEN
                '';
              };
              env = lib.mkOption {
                type = lib.types.attrs;
                default = { };
                description = ''
                  Public Environment variables to be passed to the server on startup
                '';
              };
            };
            config = lib.mkIf cfg.enable {
              nixpkgs.overlays = [ self.overlays.default ];
              users = {
                users.discord_bot = {
                  isSystemUser = true;
                  group = "discord_bot";
                };
                groups.discord_bot = {};
              };
              systemd.services = {
                discord_bot = {
                  serviceConfig.User = "discord_bot";
                  wantedBy = [ "multi-user.target" ];
                  script = ''
                    export $(cat ${cfg.env_file} | xargs)
                    export RUST_BACKTRACE=full
                    export SSH_PATH=${lib.getExe pkgs.openssh}
                    ${lib.concatStringsSep "\n" (
                      lib.mapAttrsToList (
                        name: value: "export ${name}=${toString value}"
                      ) cfg.env
                    )}
                    ${pkgs.discord_bot}/bin/server
                  '';
                };
              };
            };
          };
      };

      perSystem =
        {
          self',
          system,
          pkgs,
          lib,
          config,
          ...
        }:
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.rust-overlay.overlays.default
            ];
          };
          formatter = pkgs.nixfmt-rfc-style;

          packages = rec {
            rustToolchain = (
              pkgs.rust-bin.stable.latest.default.override {
                extensions = [
                  "rust-src"
                  "rust-analyzer"
                  "clippy"
                ];
                targets = [ "wasm32-unknown-unknown" ];
              }
            );
            wasm-bindgen-cli = pkgs.wasm-bindgen-cli.override {
              version = "0.2.99";
              hash = "sha256-1AN2E9t/lZhbXdVznhTcniy+7ZzlaEp/gwLEAucs6EA=";
              cargoHash = "sha256-DbwAh8RJtW38LJp+J9Ht8fAROK9OabaJ85D9C/Vkve4=";
            };
            dioxus-cli = pkgs.dioxus-cli.overrideAttrs (drv: rec {
              version = "0.6.1";
              src = pkgs.fetchCrate {
                inherit version;
                pname = drv.pname;
                hash = "sha256-mQnSduf8SHYyUs6gHfI+JAvpRxYQA1DiMlvNofImElU=";
              };
              cargoDeps = drv.cargoDeps.overrideAttrs (
                lib.const {
                  name = "${drv.cargoDeps.name}-vendor";
                  inherit src;
                  outputHash = "sha256-QiGnBoZV4GZb5MQ3K/PJxCfw0p/7qDmoE607hlGKOns=";
                }
              );
              checkFlags = drv.checkFlags ++ [ "--skip=wasm_bindgen::test" ];
            });
            discord_bot =
              let
                cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
                rev = toString (self.shortRev or self.dirtyShortRev or self.lastModified or "unknown");
              in
              pkgs.rustPlatform.buildRustPackage {
                pname = "discord_bot";
                version = "${cargoToml.package.version}-${rev}";
                src = ./.;
                strictDeps = true;
                nativeBuildInputs = with pkgs; [
                  dioxus-cli
                  rustToolchain
                  openssl
                  libiconv
                  pkg-config
                  rustPlatform.bindgenHook
                ];
                buildInputs =
                  with pkgs;
                  [
                    openssl
                    libiconv
                    pkg-config
                  ]
                  ++ lib.optionals pkgs.stdenv.isDarwin [
                    pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
                  ];
                buildPhase = ''
                  ${
                    if pkgs.stdenv.isDarwin then
                      ''
                        export HOME="$PWD/.home"
                        mkdir -p ".home/Library/Application Support/dioxus/wasm-bindgen"
                        ln -s ${lib.getExe wasm-bindgen-cli} ".home/Library/Application Support/dioxus/wasm-bindgen/wasm-bindgen-${wasm-bindgen-cli.version}"
                      ''
                    else
                      ''
                        export XDG_DATA_HOME="$PWD/.share"
                        mkdir -p .share/dioxus/wasm-bindgen
                        ln -s ${lib.getExe wasm-bindgen-cli} .share/dioxus/wasm-bindgen/wasm-bindgen-${wasm-bindgen-cli.version}
                      ''
                  }
                  dx build --release --platform web --verbose --trace
                '';
                installPhase = ''
                  mkdir -p $out
                  cp -r target/dx/$pname/release/web $out/bin
                '';
                meta.mainProgram = "server";
                cargoLock.lockFile = ./Cargo.lock;
              };
            default = self'.packages.discord_bot;
          };
          overlayAttrs = {
            inherit (self'.packages) discord_bot;
          };
          devShells.default = pkgs.mkShell {
            shellHook = ''
              export RUST_LOG="discord_bot=trace"
              export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}
            '';
            buildInputs = lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ];
            nativeBuildInputs = with pkgs; [
              dioxus-cli
              rustToolchain
              pkg-config
              rustPlatform.bindgenHook
              libiconv
              cargo-watch
              systemfd
            ];
          };
        };
    };
}
