class Neo4jQuery < Formula
  desc "Query Neo4j databases, output TOON"
  homepage "https://github.com/oskarhane/neo4j-query"
  version "<VERSION>"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/oskarhane/neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-apple-darwin.tar.gz"
      sha256 "<SHA256>"
    end
    on_intel do
      url "https://github.com/oskarhane/neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-apple-darwin.tar.gz"
      sha256 "<SHA256>"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/oskarhane/neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "<SHA256>"
    end
    on_intel do
      url "https://github.com/oskarhane/neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "<SHA256>"
    end
  end

  def install
    bin.install "neo4j-query"
  end

  test do
    assert_match "neo4j-query", shell_output("#{bin}/neo4j-query --help")
  end
end
