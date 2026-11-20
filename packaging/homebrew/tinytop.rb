class Tinytop < Formula
  desc "Lightweight self-hosted system and target monitor (single binary, SQLite, web dashboard and TUI)"
  homepage "https://github.com/dev1lsconf/tinytop"
  version "1.1.0"
  license "MIT"

  # Plantilla: en macOS usa artefactos nativos cuando existan; mientras tanto, ajustar URLs/checksums al release publicado.
  # Actualiza version/sha256 en cada release según los assets publicados en GitHub Releases (v1.1.0: alinear hashes tras publicar artefactos).
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-aarch64.tar.gz"
      sha256 "af6bc04689a45d2ea9c3eafe05e283db5d6f95a63bc0f4d9b3a0e0e039cb0504"
    else
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-x86_64.tar.gz"
      sha256 "12df932db8547ee4c2e1023cfc94cefdeecd544473ad5a17482eee49b31197f6"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-aarch64.tar.gz"
      sha256 "af6bc04689a45d2ea9c3eafe05e283db5d6f95a63bc0f4d9b3a0e0e039cb0504"
    else
      url "https://github.com/dev1lsconf/tinytop/releases/download/v#{version}/tinytop-v#{version}-linux-x86_64.tar.gz"
      sha256 "12df932db8547ee4c2e1023cfc94cefdeecd544473ad5a17482eee49b31197f6"
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
