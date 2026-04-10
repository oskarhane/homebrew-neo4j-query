class Neo4jQuery < Formula
  desc "Query Neo4j databases, output TOON"
  homepage "https://github.com/oskarhane/homebrew-neo4j-query"
  version "0.8.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-apple-darwin.tar.gz"
      sha256 "822013dc2fd46d4e61ad079fe2e85bc55a0a0813c399ebd2df1f93f48606148d"
    end
    on_intel do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-apple-darwin.tar.gz"
      sha256 "4ee93a2182ac5d4257532c99fbd32ef1b651a73e7fed65c888cf3070edba5bbe"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "69b5de21a2f1d4e8e4ed855e3dca39feb4aff63bc95ef08e5fbfb0e85a472fa4"
    end
    on_intel do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "81b0314d00f6b137b158cde78a5198df1c1001a838ee13fdc790de3b8611d0fd"
    end
  end

  def install
    bin.install "neo4j-query"
  end

  test do
    assert_match "neo4j-query", shell_output("#{bin}/neo4j-query --help")
  end
end
