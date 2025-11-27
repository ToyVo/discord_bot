{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    dioxus-cli-pr.url = "github:nixos/nixpkgs?ref=pull/407060/head";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay.url = "github:oxalica/rust-overlay";
    treefmt-nix.url = "github:numtide/treefmt-nix";
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
        inputs.treefmt-nix.flakeModule
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
              services.postgresql = {
                ensureDatabases = [ "discord_bot" ];
                ensureUsers = [ {
                  name = "discord_bot";
                  ensureDBOwnership = true;
                  ensureClauses.login = true;
                } ];
              };
              users = {
                users.discord_bot = {
                  isSystemUser = true;
                  group = "discord_bot";
                };
                groups.discord_bot = { };
              };
              systemd.services = {
                discord_bot = {
                  serviceConfig.User = "discord_bot";
                  wantedBy = [ "multi-user.target" ];
                  script = ''
                    export $(cat ${cfg.env_file} | xargs)
                    export RUST_BACKTRACE=full
                    ${lib.concatStringsSep "\n" (
                      lib.mapAttrsToList (name: value: "export ${name}=${toString value}") cfg.env
                    )}
                    ${lib.getExe pkgs.discord_bot}
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
        let
          nativeBuildInputs =
            with pkgs;
            [
              inputs.dioxus-cli-pr.legacyPackages.${system}.dioxus-cli
              wasm-bindgen-cli_0_2_104
              (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              openssl
              libiconv
              pkg-config
              rustPlatform.bindgenHook
              binaryen
              diesel-cli
              diesel-cli-ext
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [ darwin.sigtool ];
          buildInputs = with pkgs; [
            openssl
            libiconv
            pkg-config
          ];
          rev = toString (self.shortRev or self.dirtyShortRev or self.lastModified or "unknown");
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.rust-overlay.overlays.default
            ];
          };

          treefmt = {
            programs = {
              nixfmt.enable = true;
              rustfmt.enable = true;
              prettier.enable = true;
            };
          };

          packages = rec {
            discord_bot = pkgs.rustPlatform.buildRustPackage rec {
              pname = "discord_bot";
              version = "${(builtins.fromTOML (builtins.readFile ./discord_bot/Cargo.toml)).package.version}-${rev}";
              src = ./.;
              strictDeps = true;
              inherit nativeBuildInputs buildInputs;
              buildPhase = ''
                dx build --package discord_bot --release --verbose --trace
              '';
              installPhase = ''
                mkdir -p $out
                cp -r target/dx/$pname/release/web $out/bin
              '';
              meta.mainProgram = pname;
              cargoLock.lockFile = ./Cargo.lock;
            };
            game_manager = pkgs.rustPlatform.buildRustPackage rec {
              pname = "game_manager";
              version = "${(builtins.fromTOML (builtins.readFile ./game_manager/Cargo.toml)).package.version}-${rev}";
              src = ./.;
              strictDeps = true;
              inherit nativeBuildInputs buildInputs;
              buildPhase = ''
                dx build --package game_manager --release --verbose --trace
              '';
              installPhase = ''
                mkdir -p $out
                cp -r target/dx/$pname/release/web $out/bin
              '';
              meta.mainProgram = pname;
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
            inherit nativeBuildInputs;
          };
        };
    };
}
