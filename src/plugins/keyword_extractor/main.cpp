#include <iostream>
#include "./stop_words.hpp"

using namespace std;

void only_keep_characters(std::string& text)
{
    if (text.length() == 0) {
        return;
    }
    unsigned int i = 0;
    bool last_was_space = false;
    for (const char x : text) {
        if (x >= 'a' && x <= 'z') {
            text[i] = x;
            i++;
            last_was_space = false;
        } else if (x >= 'A' && x <= 'Z') {
            text[i] = std::tolower(x);
            i++;
            last_was_space = false;
        } else if (x >= '0' && x <= '9') {
            text[i] = x;
            i++;
            last_was_space = false;
        } else {
            if (i > 0 && !last_was_space) {
                text[i] = ' ';
                last_was_space = true;
                i++;
            }
        }
    }
    if (i > 0 && text[i - 1] == ' ') {
        i--;
    }
    text.resize(i);
}

int main()
{
    std::string test_readme = R"""(
    
    
    
 # Zigistry

**A place where you can find all the libraries and programs that suit your Zig
lang needs.**

---

## Adding your repo to Zigistry:

Either on GitHub or on Codeberg:

Go to your Repository:

Add `zig-package` topic to it, if it is a Zig library. Or Add `zig`
topic to it, if it is a Zig application/program.

## Contribution:

- Feel free to create a Pull request, mention an issue or suggest any features
  that you want Zigistry should have or improve.

## Code of conduct:

- Zigistry follows the
  [Zig Code of Conduct](https://ziglang.org/code-of-conduct/)
   
    
    
    
    )""";

    only_keep_characters(test_readme);

    std::cout << test_readme << std::endl;

    return 0;
}