# Homebrew formula for RustFig. The RustFig repository is its own tap:
#   brew tap fabianhassler/rustfig https://github.com/FabianHassler/rustfig
#   brew install --HEAD rustfig
# fig2dev is built in: `rustfig fig2dev ARGS…` runs its command line.
class Rustfig < Formula
  desc "Drawing program reimplementing xfig, with fig2dev built in"
  homepage "https://github.com/FabianHassler/rustfig"
  license any_of: ["MIT", "Apache-2.0"]
  head "https://github.com/FabianHassler/rustfig.git", branch: "main"

  depends_on "rust" => :build

  on_linux do
    depends_on "pkgconf" => :build
    depends_on "gtk+3"
    depends_on "libxcb"
    depends_on "libxkbcommon"
    depends_on "wayland"
  end

  def install
    system "cargo", "install", *std_cargo_args(path: "apps/rustfig")
    system "cargo", "install", *std_cargo_args(path: "apps/cli")
  end

  test do
    (testpath/"box.fig").write <<~FIG
      #FIG 3.2
      Landscape
      Center
      Inches
      Letter
      100.00
      Single
      -2
      1200 2
      2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5
      \t 0 0 1200 0 1200 1200 0 1200 0 0
    FIG
    system bin/"rustfig-cli", "info", "box.fig"
    system bin/"rustfig", "fig2dev", "-L", "svg", "box.fig", "box.svg"
    assert_match "<svg", (testpath/"box.svg").read
  end
end
