     ______       _     _               __     _______ 
    |__  (_) __ _(_)___| |_ _ __ _   _  \ \   / /___ / 
      / /| |/ _` | / __| __| '__| | | |  \ \ / /  |_ \ 
     / /_| | (_| | \__ \ |_| |  | |_| |   \ V /  ___) |
    /____|_|\__, |_|___/\__|_|   \__, |    \_/  |____/ 
            |___/                |___/                 



Ok, this time within the architecture, I'll be using some concepts I learn from DSA.
I'll also be using Openmp for parallelizing requests.
And this will be distributed architecture to make sure, I am
able to index every single repository since the Zig launch date, and
updating them properly would be the top priority.
on any repository. In the V2 architecture, I was range based indexing,
that will work, until someone's repository goes out of range and is not updated
within time. Range based indexing is also causing majour setbacks in terms of
code complexity and too many files. I think indexing could be made much more
easier by indexing all repos every time using ranges. And using epoch is much
much easier than compaired to the old month date date thing like DD-MM-YYYY.