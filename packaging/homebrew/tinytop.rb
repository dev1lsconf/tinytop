class Tinytop < Formula
  desc "Lightweight self-hosted system and target monitor (single binary, SQLite, web dashboard and TUI)"
  homepage "https://github.com/dev1lsconf/tinytop"
  version "0.1.0"
  license "MIT"

  # Plantilla: en macOS usa artefactos nativos cuando existan; mientras tanto, ajustar URLs/checksums al release publicado.
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-aarch64.tar.gz"
      sha256 "20eb5a90c8e67d1ebdcdeef0dd4b5ab2a4a84cb47d0074d1eceefe2960112ebc"
    else
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-x86_64.tar.gz"
      sha256 "fffa68631c73b371f35ae760583fb51d8660a56081b3826db9d808dc66a72f2c"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-aarch64.tar.gz"
      sha256 "20eb5a90c8e67d1ebdcdeef0dd4b5ab2a4a84cb47d0074d1eceefe2960112ebc"
    else
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-x86_64.tar.gz"
      sha256 "fffa68631c73b371f35ae760583fb51d8660a56081b3826db9d808dc66a72f2c"
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
