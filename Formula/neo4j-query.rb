class Neo4jQuery < Formula
  desc "Query Neo4j databases, output TOON"
  homepage "https://github.com/oskarhane/homebrew-neo4j-query"
  version "0.3.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-apple-darwin.tar.gz"
      sha256 "a27bd733f479f78686b89d98c79a2417d03c7aa71bfd9e51c1b3ba31a7a93ece"
    end
    on_intel do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-apple-darwin.tar.gz"
      sha256 "77a8b80106ac35af0ba5817b157e363404d8ce8ff776a07804b1cc26b133bfaf"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "d2fadfccb99aa192760b33c5bffa7a34222d3682ca7e9a36567aeb28ce90231e"
    end
    on_intel do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "375062a50cdf05294a48cdb5b66588e63a6e37e1444f68240fc60727fc7571c0"
    end
  end

  def install
    bin.install "neo4j-query"
  end

  test do
    assert_match "neo4j-query", shell_output("#{bin}/neo4j-query --help")
  end
end
