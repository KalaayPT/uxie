{
  description = "Uxie CLI and library packaging";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
  };

  inputs.self.submodules = true;

  outputs = { self, nixpkgs }:
    let
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in rec {
          uxie = pkgs.rustPlatform.buildRustPackage {
            pname = cargoToml.package.name;
            version = cargoToml.package.version;
            src = self;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = with pkgs; [
              gcc
              gnumake
            ];

            installPhase = ''
              runHook preInstall

              mkdir -p $out/bin $out/share/licenses/uxie
              cp target/release/uxie $out/bin/
              cp target/release/libnitroarc_ffi.so $out/bin/
              cp LICENSE $out/share/licenses/uxie/LICENSE
              cp nitroarc/COPYING $out/share/licenses/uxie/COPYING
              cp nitroarc/COPYING.LESSER $out/share/licenses/uxie/COPYING.LESSER

              runHook postInstall
            '';

            meta = with pkgs.lib; {
              description = cargoToml.package.description;
              homepage = "https://github.com/KalaayPT/uxie";
              license = [ licenses.mit licenses.lgpl3Plus ];
              mainProgram = "uxie";
              platforms = platforms.linux;
            };
          };

          default = uxie;
        });

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.uxie}/bin/uxie";
        };
      });
    };
}
