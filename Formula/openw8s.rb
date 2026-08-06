# typed: false
# frozen_string_literal: true

# Homebrew formula for openw8s.
#
#   brew install --formula Formula/openw8s.rb
#
# Optional tap:
#   brew tap almanzalex/openw8s https://github.com/almanzalex/openw8s
#   brew install openw8s

class Openw8s < Formula
  desc "Open Weights Spec CLI — inspect models, validate manifests, run environments"
  homepage "https://github.com/almanzalex/openw8s"
  version "0.1.2"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/almanzalex/openw8s/releases/download/v0.1.2/openw8s-aarch64-apple-darwin.tar.gz"
      sha256 "d5ece1a08ee9f2a75b604c9eceede417a0890793d9f019fea15d9254d7b61043"
    end
    on_intel do
      # Prefer source build until the Intel binary release is confirmed.
      depends_on "rust" => :build
      url "https://github.com/almanzalex/openw8s/archive/refs/tags/v0.1.2.tar.gz"
      sha256 "12ec64420e8e73bd1a1f936d6f8f04c7ebd3ffd9f49ef3df42d198cc132b48ca"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/almanzalex/openw8s/releases/download/v0.1.2/openw8s-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "1488b78c1357b4a8ebdb9882e70b7b59d73fce7d43df8d2b27f12d2a49b906ac"
    end
  end

  def install
    if build.head? || (OS.mac? && Hardware::CPU.intel?)
      system "cargo", "install", "--locked", "--root", prefix, "--path", "crates/openw8s-cli"
    else
      bin.install "openw8s"
    end
  end

  test do
    assert_match "openw8s", shell_output("#{bin}/openw8s --help")
  end
end
