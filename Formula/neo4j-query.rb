class Neo4jQuery < Formula
  desc "Query Neo4j databases, output TOON"
  homepage "https://github.com/oskarhane/homebrew-neo4j-query"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-apple-darwin.tar.gz"
      sha256 "302b8207882ae92a59ce9e9c8e0b83fe08505b91b2cceeb03d2392406df2d15c"
    end
    on_intel do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-apple-darwin.tar.gz"
      sha256 "41b4952d32d8536fd5841058a55b5570e62d2e4317a9cb5e06cf8794bbe4687f"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "a32700849512a8a8d510bd712891d2fc8df6ec49848ae6c8cb3765f96a8f7333"
    end
    on_intel do
      url "https://github.com/oskarhane/homebrew-neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "590d519987c25b349f568f0f1fdbbccc9416697f52d775ecbbcfbc4bf851898c"
    end
  end

  def install
    bin.install "neo4j-query"
  end

  test do
    assert_match "neo4j-query", shell_output("#{bin}/neo4j-query --help")
  end
end
