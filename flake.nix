{
  description = "Uxie CLI and library packaging";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    nitroarc = {
      url = "git+https://codeberg.org/Kalaay/nitroarc.git?ref=ffi";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, nitroarc }:
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

            postPatch = ''
              rm -rf nitroarc
              cp -r ${nitroarc} nitroarc
              chmod -R u+w nitroarc
            '';

            nativeBuildInputs = with pkgs; [
              gcc
              gnumake
            ];

            cargoTestFlags = [ "--lib" "--bins" "--tests" ];

            installPhase = ''
              runHook preInstall

              bin_path="$(find target -type f -path '*/release/uxie' -print -quit)"
              if [ -z "$bin_path" ]; then
                echo "could not locate built uxie binary under target/" >&2
                exit 1
              fi
              build_dir="$(dirname "$bin_path")"

              mkdir -p $out/bin $out/share/licenses/uxie
              cp "$build_dir/uxie" $out/bin/
              cp "$build_dir/libnitroarc_ffi.so" $out/bin/
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
