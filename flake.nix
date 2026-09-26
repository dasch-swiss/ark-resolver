{
  description = "ark-resolver (dev shell only; build is Bazel-driven)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachSystem
      [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ]
      (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          devShells.default = pkgs.mkShell {
            # cargo/rustc and python are intentionally NOT here. rules_rust/
            # rules_python in MODULE.bazel pin hermetic toolchains, so a
            # parallel host toolchain would risk version skew.
            packages = with pkgs; [
              bazelisk
              (writeShellScriptBin "bazel" ''exec ${bazelisk}/bin/bazelisk "$@"'')
              just
              uv
              cargo-audit
              cacert
            ];

            shellHook = ''
              export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
            '';
          };
        }
      );
}
