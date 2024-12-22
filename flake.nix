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
        in
        {
          packages = {
            discord_bot = pkgs.rustPlatform.buildRustPackage {
              pname = "discord_bot";
              version = "${cargoToml.package.version}-${rev}";
              src = ./.;
              strictDeps = true;
              nativeBuildInputs = with pkgs; [
                dioxus-cli
              ];
              buildInputs = lib.optionals pkgs.stdenv.isDarwin [ pkgs.darwin.apple_sdk.frameworks.SystemConfiguration ];
              buildPhase = ''
                dx build --release --platform web
              '';
              cargoLock.lockFile = ./Cargo.lock;
            };
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
