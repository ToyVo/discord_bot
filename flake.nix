{
  description = "discord_bot, A Rust web server including a NixOS module";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay.url = "github:oxalica/rust-overlay";
    crate2nix.url = "github:nix-community/crate2nix";

    # Development

    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-trusted-public-keys = "eigenvalue.cachix.org-1:ykerQDDa55PGxU25CETy9wF6uVDpadGGXYrFNJA3TUs=";
    extra-substituters = "https://eigenvalue.cachix.org";
    allow-import-from-derivation = true;
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      flake-parts,
      rust-overlay,
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
                  DISCORD_BOT_TOKEN
                '';
              };
              minecraft = {
                MCport = lib.mkOption {
                  type = lib.types.int;
                  default = 25565;
                  description = "Port to expose minecraft server on";
                };
                RCONPort = lib.mkOption {
                  type = lib.types.int;
                  default = 25575;
                  description = "Port to expose minecraft rcon on";
                };
                datadir = lib.mkOption {
                  type = lib.types.path;
                  description = "Path to store minecraft data";
                };
                openFirewall = lib.mkEnableOption "Open firewall for minecraft";
              };
              terraria = {
                port = lib.mkOption {
                  type = lib.types.int;
                  default = 7777;
                  description = "Port to expose terraria server on";
                };
                RestPort = lib.mkOption {
                  type = lib.types.int;
                  default = 7878;
                  description = "Port to expose terarria rest api on";
                };
                datadir = lib.mkOption {
                  type = lib.types.path;
                  description = "Path to store terraria data";
                };
                openFirewall = lib.mkEnableOption "Open firewall for terraria";
              };
            };
            config = lib.mkIf cfg.enable {
              nixpkgs.overlays = [ self.overlays.default ];
              systemd.services = {
                discord_bot = {
                  wantedBy = [ "multi-user.target" ];
                  serviceConfig = {
                    WorkingDirectory = ./discord_bot;
                  };
                  script = ''
                    export $(cat ${cfg.env_file} | xargs)
                    ${pkgs.discord_bot}/bin/discord_bot
                  '';
                };
              };
              networking.firewall = {
                allowedTCPPorts =
                  lib.optionals cfg.minecraft.openFirewall [
                    cfg.minecraft.MCport
                  ]
                  ++ lib.optionals cfg.terraria.openFirewall [
                    cfg.terraria.port
                  ];
                allowedUDPPorts = lib.optionals cfg.terraria.openFirewall [
                  cfg.terraria.port
                ];
              };
              virtualisation.oci-containers.containers = {
                minecraft = {
                  image = "docker.io/itzg/minecraft-server:java17";
                  # I plan to make a web interface that I want to be able to use RCON to get information but keep it internal
                  ports = [
                    "${toString cfg.minecraft.MCport}:25565"
                    "${toString cfg.minecraft.RCONPort}:25575"
                  ];
                  environment = {
                    EULA = "TRUE";
                    TYPE = "FORGE";
                    VERSION = "1.20.6";
                    MEMORY = "20g";
                    OPS = "4cb4aff4-a0ed-4eaf-b912-47825b2ed30d";
                    EXISTING_OPS_FILE = "MERGE";
                    MOTD = "ToyVo Modded Server";
                    MAX_TICK_TIME = "-1";
                    PACKWIZ_URL = "https://mc.toyvo.dev/modpack/pack.toml";
                    SPAWN_PROTECTION = "0";
                    MAX_PLAYERS = "10";
                    CREATE_CONSOLE_IN_PIPE = "true";
                    JVM_DD_OPTS = "fml.queryResult=confirm";
                    ALLOW_FLIGHT = "TRUE";
                  };
                  volumes = [
                    "${cfg.minecraft.datadir}:/data"
                  ];
                  extraOptions = [
                    "-it"
                  ];
                };
                terraria = {
                  image = "docker.io/ryshe/terraria:tshock-1.4.4.9-5.2.0-3";
                  ports = [
                    "${toString cfg.terraria.port}:7777"
                    "${toString cfg.terraria.RestPort}:7878"
                  ];
                  volumes = [
                    "${cfg.terraria.datadir}:/root/.local/share/Terraria/Worlds"
                  ];
                  environment = {
                    WORLD_FILENAME = "large_master_crimson.wld";
                  };
                };
              };
            };
          };
      };

      perSystem =
        {
          system,
          pkgs,
          lib,
          config,
          ...
        }:
        let
          generatedCargoNix = crate2nix.tools.${system}.generatedCargoNix {
            name = "discord_bot";
            src = ./.;
          };
          cargoNix = pkgs.callPackage "${generatedCargoNix}/default.nix" {
            buildRustCrateForPkgs =
              pkgs:
              pkgs.buildRustCrate.override {
                defaultCrateOverrides = pkgs.defaultCrateOverrides // {
                  discord_bot = attrs: {
                    buildInputs =
                      with pkgs.darwin.apple_sdk.frameworks;
                      lib.optionals pkgs.stdenv.isDarwin [
                        SystemConfiguration
                        CoreServices
                      ];
                    nativeBuildInputs = with pkgs; [
                      libiconv
                      pkg-config
                      openssl
                    ];
                    OPENSSL_NO_VENDOR = 1;
                    OPENSSL_LIB_DIR = "${lib.getLib pkgs.openssl}/lib";
                    OPENSSL_DIR = "${lib.getDev pkgs.openssl}";
                  };
                };
              };
          };
        in
        rec {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            overlays = [
              (import rust-overlay)
              (
                final: prev:
                assert !(prev ? rust-toolchain);
                rec {
                  rust-toolchain = (prev.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
                    extensions = [
                      "rust-src"
                      "rust-std"
                      "rust-analyzer"
                      "rustfmt"
                      "clippy"
                    ];
                  };

                  rustc = rust-toolchain;
                  cargo = rust-toolchain;
                  rustfmt = rust-toolchain;
                  clippy = rust-toolchain;
                  rust-analyzer = rust-toolchain;
                }
              )
            ];
            config = { };
          };

          packages = {
            discord_bot = cargoNix.workspaceMembers.discord_bot.build;
            default = packages.discord_bot;
          };
          overlayAttrs = {
            inherit (packages) discord_bot;
          };
          devshells.default = {
            imports = [
              "${devshell}/extra/language/c.nix"
              # "${devshell}/extra/language/rust.nix"
            ];

            env = [
              {
                name = "RUST_LOG";
                value = "discord_bot=trace";
              }
              {
                name = "RUST_SRC_PATH";
                value = "${pkgs.rust-toolchain}/lib/rustlib/src/rust/library";
              }
            ];

            commands = with pkgs; [
              {
                package = rust-toolchain;
                category = "rust";
              }
            ];

            language.c = {
              libraries = lib.optional pkgs.stdenv.isDarwin pkgs.libiconv;
            };
          };
        };
    };
}
