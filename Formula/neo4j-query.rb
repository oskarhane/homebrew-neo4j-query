class Neo4jQuery < Formula
  desc "Query Neo4j databases, output TOON"
  homepage "https://github.com/oskarhane/homebrew-neo4j-query"
  version "0.9.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-apple-darwin.tar.gz"
      sha256 "f59a5bbaa7cc418d205cceba5b9d4be230c61f7288be92f5f5eb4004807ffd1d"
    end
    on_intel do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-apple-darwin.tar.gz"
      sha256 "472dd628fd05e3ae4a9a8dcd2afa5e4b2307f51001a0ae3bc7143029c90d2b31"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "36c27515efdea499165b2dfad6fbf53016d91ee5dfe561a4c3af7e35bb86c027"
    end
    on_intel do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "1720660a0d2aa56e0b72a01df600cf30207f7482092de95b2325c5beb7663b90"
    end
  end

  def install
    bin.install "neo4j-query"
  end

  test do
    assert_match "neo4j-query", shell_output("#{bin}/neo4j-query --help")
  end
end
