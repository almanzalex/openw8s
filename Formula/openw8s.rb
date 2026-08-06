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
  version "0.1.3"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/almanzalex/openw8s/releases/download/v0.1.3/openw8s-aarch64-apple-darwin.tar.gz"
      sha256 "a4da26dd2e530c8004a1fd1ae89e9174e01f17f317f9ede8a44f97a522aeae69"
    end
    on_intel do
      url "https://github.com/almanzalex/openw8s/releases/download/v0.1.3/openw8s-x86_64-apple-darwin.tar.gz"
      sha256 "4205836ad41d30461cee4cff8988f503fa977fcb2af49dabdb3bf47d9f257ab2"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/almanzalex/openw8s/releases/download/v0.1.3/openw8s-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "1f6fd080a4a1918bd88089989d93ea6cc3709fdf568bfba63d29cb4c8de40f14"
    end
  end

  def install
    bin.install "openw8s"
  end

  test do
    assert_match "openw8s", shell_output("#{bin}/openw8s --help")
  end
end
