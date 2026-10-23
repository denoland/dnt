{
  description = "Dnt dev";

  nixConfig.bash-prompt = "[dnt-dev]> ";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs";
  };

  outputs = inputs@{ self, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems =
        [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      perSystem = { config, self', inputs', pkgs, ... }: {
        # Per-system attributes can be defined here. The self' and inputs'
        # module parameters provide easy access to attributes of the same
        # system.

        # NOTE: You can also use `config.pre-commit.devShell`
        devShells.default = pkgs.mkShell {
          shellHook = ''
            echo 1>&2 "Welcome to the DNT development shell!"
          '';
          buildInputs = with pkgs; [
            deno
            rustup
            cargo
          ];
        };
      };
    };
}
