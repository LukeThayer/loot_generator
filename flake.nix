{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo-watch
            cargo-edit
            git
          ];

          shellHook = ''
            echo "Rust development environment loaded"
            echo "Rust version: $(rustc --version)"

            # Git-aware prompt
            git_info() {
              local branch=$(git symbolic-ref --short HEAD 2>/dev/null)
              if [ -n "$branch" ]; then
                local status=""
                git diff --quiet 2>/dev/null || status="*"
                git diff --cached --quiet 2>/dev/null || status="$status+"
                echo "($branch$status)"
              fi
            }
            export PS1='\[\033[1;34m\]\w\[\033[0m\] \[\033[1;32m\]$(git_info)\[\033[0m\] \$ '
          '';
        };
      }
    );
}
