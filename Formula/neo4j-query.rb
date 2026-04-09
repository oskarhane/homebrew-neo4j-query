class Neo4jQuery < Formula
  desc "Query Neo4j databases, output TOON"
  homepage "https://github.com/oskarhane/homebrew-neo4j-query"
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-apple-darwin.tar.gz"
      sha256 "3fc27bed782a716e74ceec8eeca3891767cdaa9fc54f15b7adb193a2c5f3fd0f"
    end
    on_intel do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-apple-darwin.tar.gz"
      sha256 "661c42dcae34b4732c180fc31452006330223ad63ef2f17b2662ae04200a7732"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "31d74c2575fe7f59e579f4c251597a5c746891b08407bfaf44ac6b210afa9628"
    end
    on_intel do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "5183da1ccb0cd218a2a789f34f69a9eade4adcf1b91b75eca3d31e147d1068e6"
    end
  end

  def install
    bin.install "neo4j-query"
  end

  test do
    assert_match "neo4j-query", shell_output("#{bin}/neo4j-query --help")
  end
end
