# SHA-2
This is a toy implementation of the [SHA-224, SHA-256] digest algorithms, implemented in Rust.

There are a few simple arguments to the program:

    ~/code/sha-2 ~>> cargo build --release
    Finished release [optimized] target(s) in 0.03s

    ~/code/sha-2 ~>> ./target/release/sha-2 --test
    tests completed successfully!

    ~/code/sha-2 ~>> ./target/release/sha-2 --string abcde
    no algorithim specified; assuming SHA-256
    36bbe50ed96841d10443bcb670d6554f0a34b761be67ec9c4a8ad2c0c44ca42c

    ~/code/sha-2 ~>> echo -n abcde > input_file.txt
    ~/code/sha-2 ~>> ./target/release/sha-2 --path input_file.txt --algo 256
    36bbe50ed96841d10443bcb670d6554f0a34b761be67ec9c4a8ad2c0c44ca42c

    ~/code/sha-2 ~>> ./target/release/sha-2 --path input_file.txt --algo 224
    bdd03d560993e675516ba5a50638b6531ac2ac3d5847c61916cfced6

I tested the performance of this code against the built-in `shasum` command-line tool in OSX using the [2006 English Wikipedia Corpus](http://mattmahoney.net/dc/textdata.html), whose size comes in around ~954Mb.

    ~/code/sha-2 ~>> time ./target/release/sha-2 --path ~/Downloads/wiki/enwik9 --algo 256
    159b85351e5f76e60cbe32e04c677847a9ecba3adc79addab6f4c6c7aa3744bc

    real	0m4.413s
    user	0m4.216s
    sys	  0m0.254s

    ~/code/sha-2 ~>> time shasum -a 256 ~/Downloads/wiki/enwik9
    159b85351e5f76e60cbe32e04c677847a9ecba3adc79addab6f4c6c7aa3744bc  /Users/mike/Downloads/wiki/enwik9

    real	0m3.003s
    user	0m2.856s
    sys	  0m0.139s

This code comes in around 132% of the digest time of that tool.    