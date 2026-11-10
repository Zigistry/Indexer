CXX = clang++
CXXFLAGS = -std=c++23 -Wall -Wextra

download_dependencies:
	mkdir -p ./libs/
	curl -L https://sqlite.org/2026/sqlite-amalgamation-3540100.zip -o ./libs/sqlite.zip
	unzip ./libs/sqlite.zip -d ./libs
	mv ./libs/sqlite-amalgamation-3540100/ ./libs/sqlite/
	rm ./libs/sqlite.zip

format:
	clang-format -i --style=WebKit src/**/*.cpp
	clang-format -i --style=WebKit src/**/*.h
	clang-format -i --style=WebKit src/**/*.hpp

GH_Indexer: ./src/GH/indexer.cpp
	$(CXX) $(CXXFLAGS) ./src/GH/indexer.cpp -o ./build/main

clean:
	rm -f main