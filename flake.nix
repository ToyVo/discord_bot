{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    crate2nix.url = "github:nix-community/crate2nix";
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    arion = {
      url = "github:hercules-ci/arion";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-substituters = [
      "https://cache.nixos.org"
      "https://nix-community.cachix.org"
      "https://toyvo.cachix.org"
      "https://eigenvalue.cachix.org"
    ];
    extra-trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "toyvo.cachix.org-1:s++CG1te6YaS9mjICre0Ybbya2o/S9fZIyDNGiD4UXs="
      "eigenvalue.cachix.org-1:ykerQDDa55PGxU25CETy9wF6uVDpadGGXYrFNJA3TUs="
    ];
    allow-import-from-derivation = true;
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      flake-parts,
      crate2nix,
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
              rclone_conf_file = lib.mkOption {
                type = lib.types.path;
                description = "Path to the rclone config file";
              };
            };
            config = lib.mkIf cfg.enable {
              nixpkgs.overlays = [ self.overlays.default ];
              services.surrealdb.enable = lib.mkDefault true;
              systemd.services = {
                discord_bot = {
                  before = [ "arion-minecraft-modded.service" ];
                  requiredBy = [ "arion-minecraft-modded.service" ];
                  after = [ "surrealdb.service" ];
                  requires = [ "surrealdb.service" ];
                  wantedBy = [ "multi-user.target" ];
                  serviceConfig = {
                    WorkingDirectory = ./.;
                  };
                  script = ''
                    export $(cat ${cfg.env_file} | xargs)
                    export RUST_BACKTRACE=full
                    export RCLONE_CONF_FILE=${cfg.rclone_conf_file}
                    ${pkgs.discord_bot}/bin/discord_bot
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
          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          rev = self.shortRev or self.dirtyShortRev or "dirty";
          generatedCargoNix = crate2nix.tools.${system}.generatedCargoNix {
            name = "discord_bot";
            src = ./.;
          };
          cargoNix = pkgs.callPackage "${generatedCargoNix}/default.nix" {
            rootFeatures = [
              "db"
              "watchers"
              "backups"
            ];
            buildRustCrateForPkgs =
              pkgs:
              pkgs.buildRustCrate.override {
                defaultCrateOverrides = pkgs.defaultCrateOverrides // {
                  discord_bot =
                    attrs:
                    let
                      runtimeDeps = with pkgs; [
                        gnutar
                        rclone
                        zstd
                        uutils-coreutils-noprefix
                      ];
                    in
                    {
                      version = "${cargoToml.package.version}-${rev}";
                      postInstall = ''
                        wrapProgram $out/bin/discord_bot \
                          --prefix PATH : ${lib.makeBinPath runtimeDeps}
                      '';
                      buildInputs =
                        with pkgs.darwin.apple_sdk.frameworks;
                        lib.optionals pkgs.stdenv.isDarwin [
                          CoreServices
                          SystemConfiguration
                        ];
                      nativeBuildInputs =
                        with pkgs;
                        [
                          installShellFiles
                          makeBinaryWrapper
                          libiconv
                          openssl
                          pkg-config
                          rustPlatform.bindgenHook
                        ]
                        ++ runtimeDeps;
                      OPENSSL_NO_VENDOR = 1;
                      OPENSSL_LIB_DIR = "${lib.getLib pkgs.openssl}/lib";
                      OPENSSL_DIR = "${lib.getDev pkgs.openssl}";
                    };
                };
              };
          };
        in
        {
          packages = {
            discord_bot = cargoNix.rootCrate.build;
            default = self'.packages.discord_bot;
          };
          overlayAttrs = {
            inherit (self'.packages) discord_bot;
          };
          devShells.default =
            let
              dev_start = pkgs.writeShellScriptBin "dev_start" ''
                systemfd --no-pid -s http::8080 -- cargo watch -x run
              '';
            in
            pkgs.mkShell {
              shellHook = ''
                export RUST_LOG="discord_bot=trace"
                export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}
              '';
              buildInputs =
                with pkgs.darwin.apple_sdk.frameworks;
                lib.optionals pkgs.stdenv.isDarwin [ SystemConfiguration ];
              nativeBuildInputs = with pkgs; [
                rustc
                pkg-config
                rustPlatform.bindgenHook
                libiconv
                cargo-watch
                systemfd
                dev_start
              ];
            };
        };
    };
}
