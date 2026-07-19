{
  description = "Paddler — open-source LLMOps platform for hosting and scaling LLMs in your own infrastructure";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      flake-parts,
      rust-overlay,
      crane,
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem =
        { system, ... }:
        let
          allowUnfree = builtins.getEnv "NIXPKGS_ALLOW_UNFREE" == "1";
          cudaSupport = builtins.getEnv "PADDLER_ENABLE_CUDA" == "1";

          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
            config = { inherit allowUnfree cudaSupport; };
          };
          lib = pkgs.lib;

          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
          craneLibCuda = craneLib.overrideScope (
            _final: _prev: {
              stdenvSelector = eachPkgs: eachPkgs.cudaPackages.backendStdenv;
            }
          );

          version = "4.1.0";

          webAdminPanelAssets = pkgs.buildNpmPackage {
            pname = "paddler-web-admin-panel";
            inherit version;
            src = self;
            npmDepsHash = "sha256-sBDdMf388qFQVIjQ3t/BL3KC/yAqF1qj47a/40axgF8=";
            dontNpmBuild = true;
            nativeBuildInputs = [ pkgs.nodejs ];
            buildPhase = ''
              runHook preBuild
              node jarmuz-static.mjs
              runHook postBuild
            '';
            installPhase = ''
              runHook preInstall
              mkdir -p "$out"
              cp -r static "$out/static"
              cp esbuild-meta.json "$out/esbuild-meta.json"
              runHook postInstall
            '';
          };

          injectWebAdminPanelAssets = ''
            cp -r --no-preserve=mode,ownership ${webAdminPanelAssets}/static ./static
            cp --no-preserve=mode,ownership ${webAdminPanelAssets}/esbuild-meta.json ./esbuild-meta.json
          '';

          acceleratorInputs =
            accelerator:
            if accelerator == "cpu" then
              {
                cargoFeatures = [ ];
                nativeBuildInputs = [ ];
                buildInputs = [ ];
                env = { };
              }
            else if accelerator == "cuda" then
              {
                cargoFeatures = [ "cuda" ];
                nativeBuildInputs = [
                  pkgs.cudaPackages.cuda_nvcc
                  pkgs.autoAddDriverRunpath
                ];
                buildInputs = [
                  pkgs.cudaPackages.cuda_cudart
                  pkgs.cudaPackages.libcublas
                  (lib.getOutput "static" pkgs.cudaPackages.libcublas)
                  pkgs.cudaPackages.cccl
                ];
                env = {
                  CMAKE_CUDA_ARCHITECTURES = pkgs.cudaPackages.flags.cmakeCudaArchitecturesString;
                  CARGO_BUILD_RUSTFLAGS = lib.concatStringsSep " " [
                    "-L native=${pkgs.cudaPackages.cuda_cudart}/lib"
                    "-L native=${pkgs.cudaPackages.cuda_cudart}/lib/stubs"
                    "-L native=${lib.getOutput "static" pkgs.cudaPackages.libcublas}/lib"
                  ];
                };
              }
            else if accelerator == "metal" then
              {
                cargoFeatures = [ "metal" ];
                nativeBuildInputs = [ ];
                buildInputs = [ ];
                env = { };
              }
            else
              throw "paddler: unsupported accelerator '${accelerator}'";

          mkPaddler =
            {
              accelerator ? "cpu",
              webAdminPanel ? true,
            }:
            let
              accel = acceleratorInputs accelerator;
              craneLibEff = if accelerator == "cuda" then craneLibCuda else craneLib;
              features = accel.cargoFeatures ++ lib.optional webAdminPanel "web_admin_panel";
              featureFlags = lib.optionals (features != [ ]) [
                "--features"
                (lib.concatStringsSep "," features)
              ];
              cargoExtraArgs = lib.escapeShellArgs (
                [
                  "-p"
                  "paddler_cli"
                ]
                ++ featureFlags
              );

              pname = "paddler${lib.optionalString (accelerator != "cpu") "-${accelerator}"}${
                lib.optionalString (!webAdminPanel) "-headless"
              }";

              commonArgs = {
                inherit cargoExtraArgs version pname;
                src = self;
                strictDeps = true;
                doCheck = false;
                nativeBuildInputs = [
                  pkgs.cmake
                  pkgs.pkg-config
                  pkgs.llvmPackages.clang
                ]
                ++ accel.nativeBuildInputs;
                buildInputs = [ pkgs.openssl ] ++ accel.buildInputs;
                LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
              }
              // accel.env;

              cargoArtifacts = craneLibEff.buildDepsOnly (
                commonArgs // { src = craneLibEff.cleanCargoSource self; }
              );
            in
            craneLibEff.buildPackage (
              commonArgs
              // {
                inherit cargoArtifacts;
                meta = {
                  description = "Paddler ${accelerator} build (web admin panel ${
                    if webAdminPanel then "enabled" else "disabled"
                  })";
                  homepage = "https://paddler.intentee.com/";
                  license = lib.licenses.asl20;
                  mainProgram = "paddler";
                };
              }
              // lib.optionalAttrs webAdminPanel { preBuild = injectWebAdminPanelAssets; }
            );

          enableCuda = system == "x86_64-linux" && allowUnfree && cudaSupport;

          linuxPackages = {
            paddler = mkPaddler { };
            paddler-headless = mkPaddler { webAdminPanel = false; };
          }
          // lib.optionalAttrs enableCuda {
            paddler-cuda = mkPaddler { accelerator = "cuda"; };
            paddler-cuda-headless = mkPaddler {
              accelerator = "cuda";
              webAdminPanel = false;
            };
          };

          darwinPackages = {
            paddler = mkPaddler { accelerator = "metal"; };
            paddler-headless = mkPaddler {
              accelerator = "metal";
              webAdminPanel = false;
            };
          };

          accelPackages = if pkgs.stdenv.hostPlatform.isDarwin then darwinPackages else linuxPackages;
        in
        {
          packages = accelPackages // {
            default = accelPackages.paddler;
          };

          apps.default = {
            type = "app";
            program = "${lib.getExe accelPackages.paddler}";
          };

          checks = {
            paddler = accelPackages.paddler;
            paddler-headless = accelPackages.paddler-headless;
          };

          devShells.default = craneLib.devShell {
            packages = [
              pkgs.nodejs
              pkgs.cmake
              pkgs.pkg-config
              pkgs.llvmPackages.clang
              pkgs.openssl
            ];
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          };

          formatter = pkgs.nixfmt-rfc-style;
        };

      flake =
        let
          paddlerNixosModule =
            {
              config,
              lib,
              pkgs,
              utils,
              ...
            }:
            let
              cfg = config.services.paddler;

              defaultPackage = self.packages.${pkgs.stdenv.hostPlatform.system}.paddler;

              socketAddrType = lib.types.str;

              balancerArgs =
                let
                  balancer = cfg.balancer;
                in
                [
                  "balancer"
                  "--management-addr"
                  balancer.managementAddr
                  "--inference-addr"
                  balancer.inferenceAddr
                  "--state-database"
                  balancer.stateDatabase
                ]
                ++ lib.optionals (balancer.webAdminPanelAddr != null) [
                  "--web-admin-panel-addr"
                  balancer.webAdminPanelAddr
                ]
                ++ lib.optionals (balancer.openaiCompatAddr != null) [
                  "--compat-openai-addr"
                  balancer.openaiCompatAddr
                ]
                ++ lib.concatMap (host: [
                  "--management-cors-allowed-host"
                  host
                ]) balancer.managementCorsAllowedHosts
                ++ lib.concatMap (host: [
                  "--inference-cors-allowed-host"
                  host
                ]) balancer.inferenceCorsAllowedHosts
                ++ balancer.extraArgs;

              agentArgs =
                let
                  agent = cfg.agent;
                in
                [
                  "agent"
                  "--management-addr"
                  agent.managementAddr
                  "--slots"
                  (toString agent.slots)
                ]
                ++ lib.optionals (agent.name != null) [
                  "--name"
                  agent.name
                ]
                ++ agent.extraArgs;
            in
            {
              options.services.paddler = {
                balancer = {
                  enable = lib.mkEnableOption "the Paddler balancer service";

                  package = lib.mkOption {
                    type = lib.types.package;
                    default = defaultPackage;
                    defaultText = lib.literalExpression "paddler.packages.\${system}.paddler";
                    description = "The paddler package used for the balancer.";
                  };

                  managementAddr = lib.mkOption {
                    type = socketAddrType;
                    default = "127.0.0.1:8060";
                    description = ''
                      Address of the management server. Agents connect here and the web admin
                      panel calls it directly from the browser, so if the panel is used remotely
                      this must be an address the browser can actually reach.
                    '';
                  };

                  inferenceAddr = lib.mkOption {
                    type = socketAddrType;
                    default = "127.0.0.1:8061";
                    description = ''
                      Address of the inference server. The web admin panel calls it directly from
                      the browser, so if the panel is used remotely this must be browser-reachable.
                    '';
                  };

                  webAdminPanelAddr = lib.mkOption {
                    type = lib.types.nullOr socketAddrType;
                    default = null;
                    example = "127.0.0.1:8062";
                    description = ''
                      Address of the web admin panel. When null the panel is disabled. Requires a
                      package built with the web admin panel feature (the default package).
                    '';
                  };

                  openaiCompatAddr = lib.mkOption {
                    type = lib.types.nullOr socketAddrType;
                    default = null;
                    description = "Address of the OpenAI-compatible API server. When null it is disabled.";
                  };

                  stateDatabase = lib.mkOption {
                    type = lib.types.str;
                    default = "file:///var/lib/paddler/state.db";
                    description = ''
                      Balancer state database URL. Either memory:// or file:///absolute/path.
                      A file database persists the runtime model assignment across restarts.
                    '';
                  };

                  managementCorsAllowedHosts = lib.mkOption {
                    type = lib.types.listOf lib.types.str;
                    default = [ ];
                    description = "Allowed CORS hosts for the management service.";
                  };

                  inferenceCorsAllowedHosts = lib.mkOption {
                    type = lib.types.listOf lib.types.str;
                    default = [ ];
                    description = "Allowed CORS hosts for the inference service.";
                  };

                  extraArgs = lib.mkOption {
                    type = lib.types.listOf lib.types.str;
                    default = [ ];
                    description = "Extra command-line arguments passed to the balancer.";
                  };

                  openFirewall = lib.mkOption {
                    type = lib.types.bool;
                    default = false;
                    description = "Open the management, inference, web admin panel and OpenAI-compatible ports in the firewall.";
                  };
                };

                agent = {
                  enable = lib.mkEnableOption ''
                    the Paddler agent service. Run a single agent per host: one agent already
                    saturates the host's inference hardware with its slots, so additional agents
                    on the same host would contend for the same GPU or CPU
                  '';

                  package = lib.mkOption {
                    type = lib.types.package;
                    default = defaultPackage;
                    defaultText = lib.literalExpression "paddler.packages.\${system}.paddler";
                    description = "The paddler package used for the agent (e.g. paddler-cuda for GPU).";
                  };

                  managementAddr = lib.mkOption {
                    type = socketAddrType;
                    example = "127.0.0.1:8060";
                    description = "Management address of the balancer to connect to.";
                  };

                  slots = lib.mkOption {
                    type = lib.types.ints.positive;
                    example = 4;
                    description = "Number of parallel requests this agent can handle at once.";
                  };

                  name = lib.mkOption {
                    type = lib.types.nullOr lib.types.str;
                    default = null;
                    description = "Human-readable name reported to the balancer.";
                  };

                  hfTokenFile = lib.mkOption {
                    type = lib.types.nullOr lib.types.path;
                    default = null;
                    example = "/run/secrets/paddler-hf-token";
                    description = ''
                      Path to a file containing a HuggingFace access token (the raw token on a
                      single line), used to download gated repositories. It is loaded as a
                      systemd credential and installed into the agent's HuggingFace cache
                      (HF_HOME/token) before the agent starts.
                    '';
                  };

                  environment = lib.mkOption {
                    type = lib.types.attrsOf lib.types.str;
                    default = { };
                    description = "Extra environment variables for the agent process.";
                  };

                  extraArgs = lib.mkOption {
                    type = lib.types.listOf lib.types.str;
                    default = [ ];
                    description = "Extra command-line arguments passed to the agent.";
                  };
                };
              };

              config = lib.mkMerge [
                (lib.mkIf cfg.balancer.enable {
                  systemd.services.paddler-balancer = {
                    description = "Paddler balancer";
                    after = [ "network-online.target" ];
                    wants = [ "network-online.target" ];
                    wantedBy = [ "multi-user.target" ];
                    serviceConfig = {
                      ExecStart = utils.escapeSystemdExecArgs ([ (lib.getExe cfg.balancer.package) ] ++ balancerArgs);
                      DynamicUser = true;
                      StateDirectory = "paddler";
                      Restart = "on-failure";
                      RestartSec = 5;
                      ProtectSystem = "strict";
                      ProtectHome = true;
                      NoNewPrivileges = true;
                      PrivateTmp = true;
                    };
                  };

                  networking.firewall = lib.mkIf cfg.balancer.openFirewall {
                    allowedTCPPorts =
                      let
                        portOf = addr: lib.toInt (lib.last (lib.splitString ":" addr));
                      in
                      [
                        (portOf cfg.balancer.managementAddr)
                        (portOf cfg.balancer.inferenceAddr)
                      ]
                      ++ lib.optional (cfg.balancer.webAdminPanelAddr != null) (portOf cfg.balancer.webAdminPanelAddr)
                      ++ lib.optional (cfg.balancer.openaiCompatAddr != null) (portOf cfg.balancer.openaiCompatAddr);
                  };
                })

                (lib.mkIf cfg.agent.enable {
                  systemd.services.paddler-agent = {
                    description = "Paddler agent";
                    after = [ "network-online.target" ];
                    wants = [ "network-online.target" ];
                    wantedBy = [ "multi-user.target" ];
                    environment = {
                      PADDLER_CACHE_DIR = "/var/cache/paddler";
                      HF_HOME = "/var/cache/paddler/huggingface";
                    }
                    // cfg.agent.environment;
                    serviceConfig = {
                      ExecStart = utils.escapeSystemdExecArgs ([ (lib.getExe cfg.agent.package) ] ++ agentArgs);
                      DynamicUser = true;
                      CacheDirectory = "paddler";
                      Restart = "on-failure";
                      RestartSec = 5;
                      ProtectSystem = "strict";
                      ProtectHome = true;
                      NoNewPrivileges = true;
                      PrivateTmp = true;
                    }
                    // lib.optionalAttrs (cfg.agent.hfTokenFile != null) {
                      LoadCredential = [ "hf-token:${toString cfg.agent.hfTokenFile}" ];
                      ExecStartPre = "${lib.getExe' pkgs.coreutils "install"} -D -m0600 %d/hf-token /var/cache/paddler/huggingface/token";
                    };
                  };
                })
              ];
            };
        in
        {
          nixosModules.default = paddlerNixosModule;
          nixosModules.paddler = paddlerNixosModule;

          overlays.default = final: _prev: {
            paddler = self.packages.${final.stdenv.hostPlatform.system}.paddler;
            paddler-headless = self.packages.${final.stdenv.hostPlatform.system}.paddler-headless;
          };
        };
    };
}
