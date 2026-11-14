class Tinytop < Formula
  desc "Lightweight self-hosted system and target monitor (single binary, SQLite, web dashboard and TUI)"
  homepage "https://github.com/dev1lsconf/tinytop"
  version "1.0.1"
  license "MIT"

  # Plantilla: en macOS usa artefactos nativos cuando existan; mientras tanto, ajustar URLs/checksums al release publicado.
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-aarch64.tar.gz"
      sha256 "49c9fa1df394d9f9dd72c48f4a0c51baf26576e5dcf969a2a616e48daab33930"
    else
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-x86_64.tar.gz"
      sha256 "4827042a981a3af75a71c061554b945f71377bbba71c95667f35ce90b16b8318"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-aarch64.tar.gz"
      sha256 "49c9fa1df394d9f9dd72c48f4a0c51baf26576e5dcf969a2a616e48daab33930"
    else
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-x86_64.tar.gz"
      sha256 "4827042a981a3af75a71c061554b945f71377bbba71c95667f35ce90b16b8318"
    end
  end

  def install
    bin.install "tinytop"
    (etc/"tinytop").install "config.toml"
    pkgshare.install "static"
    pkgshare.install "migrations"
  end

  def caveats
    <<~EOS
      Config: #{etc}/tinytop/config.toml
      Run web dashboard: tinytop --config #{etc}/tinytop/config.toml
      Run TUI: tinytop --tui --config #{etc}/tinytop/config.toml
    EOS
  end

  test do
    assert_match "tinytop", shell_output("#{bin}/tinytop --version")
  end
end
