# osm2graph

## cmake issues

When installing rsmgclient with cargo for this project, I kept running into errors that looked like this:

```
   Compiling rsmgclient v2.0.2                                                                                                                                                                                                          
error: linking with `cc` failed: exit status: 1                                                                                                                                                                                         
  |                                                                                                                                                                                                                                     
  = note: LC_ALL="C" PATH="/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin:/home/jacob/.miniconda3/envs/geodata/bin:/home/jacob/.miniconda3/condabin:/home/jacob/.cargo/bin:/usr
/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin" VSLANG="1033" "cc" "-m64" "/tmp/rustcNvdT8W/symbols.o" "/tmp/cargo-installfhRCvC/release/deps/rsmgclient-14496c3b883a457e.rsmgclient.7f432ee03be458b5-cgu.0.rcgu.o" "/tmp/cargo-installfh
RCvC/release/deps/rsmgclient-14496c3b883a457e.rsmgclient.7f432ee03be458b5-cgu.1.rcgu.o" "/tmp/cargo-installfhRCvC/release/deps/rsmgclient-14496c3b883a457e.96yczp1n8lfp2s0.rcgu.o" "-Wl,--as-needed" "-L" "/tmp/cargo-installfhRCvC/rele
ase/deps" "-L" "/usr/local/lib64" "-L" "/usr/local/include" "-L" "/tmp/cargo-installfhRCvC/release/build/rsmgclient-b126fd40c4e03378/out/lib" "-L" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-un
known-linux-gnu/lib" "-Wl,-Bstatic" "/tmp/cargo-installfhRCvC/release/deps/librsmgclient-510da65a8f0fa236.rlib" "/tmp/cargo-installfhRCvC/release/deps/libchrono-22847e14d5031490.rlib" "/tmp/cargo-installfhRCvC/release/deps/libnum_tr
aits-571a6fb9f30eed7c.rlib" "/tmp/cargo-installfhRCvC/release/deps/libiana_time_zone-0f334b3a318c9de3.rlib" "/tmp/cargo-installfhRCvC/release/deps/libmaplit-8e598232167352d9.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknow
n-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libstd-6498d8891e016dca.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libpanic_unwind-3debdee1a9058d84.rlib" "/hom
e/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libobject-8339c5bd5cbc92bf.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gn
u/lib/libmemchr-160ebcebb54c11ba.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libaddr2line-95c75789f1b65e37.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknow
n-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libgimli-7e8094f2d6258832.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/librustc_demangle-bac9783ef1b45db0.rlib" "
/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libstd_detect-a1cd87df2f2d8e76.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-
linux-gnu/lib/libhashbrown-7fd06d468d7dba16.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/librustc_std_workspace_alloc-5ac19487656e05bf.rlib" "/home/jacob/.rustup/tool
chains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libminiz_oxide-c7c35d32cf825c11.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libadler-
c523f1571362e70b.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libunwind-85f17c92b770a911.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rus
tlib/x86_64-unknown-linux-gnu/lib/libcfg_if-598d3ba148dadcea.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/liblibc-a58ec2dab545caa4.rlib" "/home/jacob/.rustup/toolchai
ns/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/liballoc-f9dda8cca149f0fc.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/librustc_std_worksp
ace_core-7ba4c315dd7a3503.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libcore-5ac2993e19124966.rlib" "/home/jacob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/
lib/rustlib/x86_64-unknown-linux-gnu/lib/libcompiler_builtins-df2fb7f50dec519a.rlib" "-Wl,-Bdynamic" "-lcrypto" "-lssl" "-lgcc_s" "-lutil" "-lrt" "-lpthread" "-lm" "-ldl" "-lc" "-Wl,--eh-frame-hdr" "-Wl,-z,noexecstack" "-L" "/home/j
acob/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib" "-o" "/tmp/cargo-installfhRCvC/release/deps/rsmgclient-14496c3b883a457e" "-Wl,--gc-sections" "-pie" "-Wl,-z,relro,-z,now" "-Wl,-O1" "-
nodefaultlibs"                                                                                                                                                                                                                          
  = note: /usr/bin/ld: /tmp/cargo-installfhRCvC/release/deps/librsmgclient-510da65a8f0fa236.rlib(mgclient.c.o): relocation R_X86_64_32 against `.rodata.str1.1' can not be used when making a PIE object; recompile with -fPIE          
          /usr/bin/ld: failed to set dynamic section sizes: bad value                                                                                                                                                                   
          collect2: error: ld returned 1 exit status                                                                                                                                                                                    
                                                                                                                                                                                                                                        
                                                                                                                                                                                                                                        
error: could not compile `rsmgclient` (bin "rsmgclient") due to previous error                                                                                                                                                          
error: failed to compile `rsmgclient v2.0.2`, intermediate artifacts can be found at `/tmp/cargo-installfhRCvC`.                                                                                                                        
To reuse those artifacts with a future compilation, set the environment variable `CARGO_TARGET_DIR` to that path.
```

Eventually I figured out that building mgclient with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON` worked.

On my machine, I then have to run `RUSTFLAGS='-L /usr/local/lib64' cargo install rsmgclient` to get cargo to see mgclient correctly.
