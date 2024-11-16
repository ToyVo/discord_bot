{
  description = "discord_bot, A Rust web server including a NixOS module";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    crate2nix.url = "github:nix-community/crate2nix";

    # Development

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
        #        devshell.flakeModule
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
              rclone_dir = lib.mkOption {
                type = lib.types.path;
                description = "Path to the rclone config file";
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
                voicePort = lib.mkOption {
                  type = lib.types.int;
                  default = 24454;
                  description = "Port to expose minecraft simple voice chat on";
                };
                datadir = lib.mkOption {
                  type = lib.types.path;
                  description = "Path to store minecraft data";
                };
                openFirewall = lib.mkEnableOption "Open firewall for minecraft";
              };
              minecraft_geyser = {
                MCport = lib.mkOption {
                  type = lib.types.int;
                  default = 25566;
                  description = "Port to expose minecraft server on";
                };
                BedrockPort = lib.mkOption {
                  type = lib.types.int;
                  default = 19132;
                  description = "Port to expose minecraft server on";
                };
                RCONPort = lib.mkOption {
                  type = lib.types.int;
                  default = 25576;
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
              services.surrealdb.enable = lib.mkDefault true;
              systemd.services = {
                discord_bot = {
                  after = [ "surrealdb.service" ];
                  wantedBy = [ "multi-user.target" ];
                  serviceConfig = {
                    WorkingDirectory = ./.;
                  };
                  script = ''
                    export $(cat ${cfg.env_file} | xargs)
                    export RUST_BACKTRACE=full
                    ${pkgs.discord_bot}/bin/discord_bot
                  '';
                };
                arion-terraria.wantedBy = lib.mkForce [];
                arion-minecraft-geyser.wantedBy = lib.mkForce [];
              };
              networking.firewall = {
                allowedTCPPorts =
                  lib.optionals cfg.minecraft.openFirewall [
                    cfg.minecraft.MCport
                  ]
                  ++ lib.optionals cfg.minecraft_geyser.openFirewall [
                    cfg.minecraft_geyser.MCport
                  ]
                  ++ lib.optionals cfg.terraria.openFirewall [
                    cfg.terraria.port
                  ];
                allowedUDPPorts =
                  lib.optionals cfg.terraria.openFirewall [
                    cfg.terraria.port
                  ]
                  ++ lib.optionals cfg.minecraft.openFirewall [
                    cfg.minecraft.voicePort
                  ]
                  ++ lib.optionals cfg.minecraft_geyser.openFirewall [
                    cfg.minecraft_geyser.BedrockPort
                  ];
              };
              virtualisation.arion.projects = {
                minecraft-modded.settings.services = {
                  mc.service = {
                    image = "docker.io/itzg/minecraft-server:java17";
                    # I plan to make a web interface that I want to be able to use RCON to get information but keep it internal
                    ports = [
                      "${toString cfg.minecraft.MCport}:25565"
                      "${toString cfg.minecraft.RCONPort}:25575"
                      "${toString cfg.minecraft.voicePort}:24454/udp"
                    ];
                    env_file = [ cfg.env_file ];
                    environment = {
                      EULA = "TRUE";
                      TYPE = "FORGE";
                      FORGE_VERSION = "47.3.10";
                      VERSION = "1.20.1";
                      MEMORY = "20g";
                      OPS = "4cb4aff4-a0ed-4eaf-b912-47825b2ed30d";
                      EXISTING_OPS_FILE = "MERGE";
                      EXISTING_WHITELIST_FILE = "MERGE";
                      MOTD = "ToyVo Modded Server";
                      MAX_TICK_TIME = "-1";
                      PACKWIZ_URL = "https://mc.toyvo.dev/modpack/pack.toml";
                      SPAWN_PROTECTION = "0";
                      MAX_PLAYERS = "10";
                      CREATE_CONSOLE_IN_PIPE = "true";
                      JVM_DD_OPTS = "fml.queryResult=confirm";
                      ALLOW_FLIGHT = "TRUE";
                      DIFFICULTY = "hard";
                      VIEW_DISTANCE = "8";
                      SIMULATION_DISTANCE = "8";
                      MAX_CHAINED_NEIGHBOR_UPDATES="10000";
                      MAX_WORLD_SIZE="12500";
                      RATE_LIMIT="100";
                      RCON_CMDS_STARTUP = "gamerule playersSleepingPercentage 0\ngamerule mobGriefing false\ngamerule doFireTick false\ngamerule doInsomnia false";
                    };
                    volumes = [
                      "${cfg.minecraft.datadir}:/data"
                    ];
                  };
                };
                minecraft-geyser.settings.services = {
                  mc.service = {
                    image = "docker.io/itzg/minecraft-server:java17";
                    ports = [
                      "${toString cfg.minecraft_geyser.MCport}:25565"
                      "${toString cfg.minecraft_geyser.RCONPort}:25575"
                      "${toString cfg.minecraft_geyser.BedrockPort}:19132/udp"
                    ];
                    env_file = [ cfg.env_file ];
                    environment = {
                      EULA = "TRUE";
                      TYPE = "PAPER";
                      VERSION = "1.20.1";
                      MEMORY = "4g";
                      OPS = "4cb4aff4-a0ed-4eaf-b912-47825b2ed30d";
                      EXISTING_OPS_FILE = "MERGE";
                      EXISTING_WHITELIST_FILE = "MERGE";
                      MOTD = "ToyVo Geyser Server";
                      MAX_TICK_TIME = "-1";
                      SPAWN_PROTECTION = "0";
                      MAX_PLAYERS = "10";
                      CREATE_CONSOLE_IN_PIPE = "true";
                      ALLOW_FLIGHT = "TRUE";
                      DIFFICULTY = "hard";
                    };
                    volumes = [
                      "${cfg.minecraft_geyser.datadir}:/data"
                    ];
                  };
                };
                terraria.settings.services.terraria.service = {
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
            rootFeatures = ["db" "watchers" "backups"];
            buildRustCrateForPkgs =
              pkgs:
              pkgs.buildRustCrate.override {
                defaultCrateOverrides = pkgs.defaultCrateOverrides // {
                  discord_bot = attrs: {
                    version = "${cargoToml.package.version}-${rev}";
                    buildInputs =
                      with pkgs.darwin.apple_sdk.frameworks;
                      lib.optionals pkgs.stdenv.isDarwin [
                        CoreServices
                        SystemConfiguration
                      ];
                    nativeBuildInputs = with pkgs; [
                      libiconv
                      openssl
                      pkg-config
                      rustPlatform.bindgenHook
                      rclone
                      gnutar
                      zstd
                    ];
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
              buildInputs = with pkgs.darwin.apple_sdk.frameworks; lib.optionals pkgs.stdenv.isDarwin [ SystemConfiguration ];
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
          # devshells.default = {
          #   imports = [
          #     "${devshell}/extra/language/c.nix"
          #     # "${devshell}/extra/language/rust.nix"
          #   ];
          #
          #   env = [
          #     {
          #       name = "RUST_LOG";
          #       value = "discord_bot=trace";
          #     }
          #     {
          #       name = "RUST_SRC_PATH";
          #       value = "${pkgs.rustPlatform.rustLibSrc}";
          #     }
          #   ];
          #
          #   packages =
          #     with pkgs;
          #     [
          #       cargo
          #       cargo-watch
          #       clippy
          #       pkg-config
          #       rust-analyzer-unwrapped
          #       rustPlatform.bindgenHook
          #       rustc
          #       rustfmt
          #       systemfd
          #     ]
          #     ++ lib.optionals stdenv.isDarwin [ darwin.apple_sdk.frameworks.SystemConfiguration ];
          #
          #   commands = [
          #     {
          #       name = "start_dev";
          #       help = "Start axum server with listener to restart on code changes.";
          #       command = ''
          #         systemfd --no-pid -s http::8080 -- cargo watch -x run
          #       '';
          #     }
          #   ];
          #
          #   language.c = {
          #     libraries = lib.optional pkgs.stdenv.isDarwin pkgs.libiconv;
          #   };
          # };
        };
    };
}
