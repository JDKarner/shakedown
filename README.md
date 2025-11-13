stress-ng dependant

--metrics-brief
--klog-check
--yaml file
--with stressor,stressor --sequential N

--iostat??




✅ Verification Always Enabled

These stressors have their verification logic built-in and active by default.

🧠 CPU (General, Compute, Math & Sorting)

    besselmath

    bitops

    branch

    chyperbolic

    context

    cpu-online

    crypt

    ctrig

    eigen

    expmath

    factor

    far-branch

    fp-error

    funccall

    funcret

    goto

    hyperbolic

    intmath

    list

    logmath

    longjmp

    misaligned

    mpfr

    powmath

    priv-instr

    rdrand

    regs

    skiplist

    sparsematrix

    trig

    veccmp

    vecmath

    vecshuf

    vnni

    waitcpu

    x86cpuid

    x86syscall

🚦 CPU (Cache, Atomics & Mutex)

    atomic

    cachehammer

    cacheline

    dekker

    llc-affinity

    mtx

    mutex

    peterson

💾 Memory (RAM)

    brk

    dev-shm

    memfd

    numa

    physpage

    ptr-chase

    shm

    shm-sysv

    stack

    stackmmap

📄 Virtual Memory (VM) & Paging

    mincore

    mlock

    mmapaddr

    mmapfiles

    mmapfixed

    mmapfork

    mmaphuge

    mmapmany

    mprotect

    mseal

    msync

    msyncmany

    munmap

    pagemove

    remap  //'mmap: stress-ng-remap (63173) uses deprecated remap_file_pages() syscall. See Documentation/mm/remap_file_pages.rst.'


    rmap

    swap

    userfaultfd

    vm-addr

    vm-segv

💽 Storage I/O (Disk)

    aio

    aiol

    full

    io

    iomix

    ioprio

    io-uring

    loop

    null

    pseek

    rawdev

    revio

    sync-file

    tee

    vm-splice

    zero

📁 Filesystem (Metadata & Ops)

    access

    acl

    bind-mount

    chattr

    chdir

    chmod

    chown

    chroot

    close

    dir

    dirdeep

    dirmany

    dnotify

    dup

    fanotify

    fcntl

    fd-fork

    fd-race

    fiemap

    file-ioctl

    filename

    flock

    fsize

    fstat

    getdent

    handle

    lease

    link

    locka

    lockf

    lockmix

    lockofd

    mknod

    quota

    ramfs

    rename

    seal

    statmount

    symlink

    touch

    umask

    umount

    verity

    xattr

🌐 Network

    dccp

    epoll

    icmp-flood

    netdev

    netlink-task

    rawpkt

    rawsock

    rawudp

    sctp

    sendfile

    sock

    sockabuse

    sockdiag

    sockfd

    sockmany

    tun

    udp

    udp-flood

🏭 Kernel (Process, Scheduling & Unsharing)

    clone

    cpu-sched

    daemon

    exit-group

    nice

    pidfd

    prctl

    prio-inv

    pthread

    ptrace

    race-sched

    resched

    rlimit

    schedmix

    schedpolicy

    session

    switch

    unshare

    vforkmany

    wait

    workload

🔔 Kernel (Signals)

    bad-altstack

    sigabrt

    sigchld

    sighup

    sigio

    signal

    signest

    sigpending

    sigpipe

    sigq

    sigrt

    sigsuspend

    sigtrap

    sigvtalrm

    sigxcpu

    sigxfsz

📨 Kernel (IPC, Pipes & Semaphores)

    binderfs

    eventfd

    fifo

    msg

    oom-pipe

    pipeherd

    sem

    sem-sysv

⏰ Kernel (Timers & Clocks)

    hrtimers

    itimer

    min-nanosleep

    nanosleep

    sleep

    timer

    timerfd

    timermix

    time-warp

🛂 Kernel (Syscalls & Security)

    apparmor

    cap

    cgroup

    getrandom

    key

    klog

    landlock

    personality

    reboot

    seccomp

    set

    softlockup

    usersyscall

🔌 I/O & Peripherals

    efivar

    ioport

    kvm

    pty

    rtc

    smi

    watchdog

🔬 Verification Enabled by --verify Option

These stressors will only perform verification checks when you explicitly add the --verify flag to your command.

🧠 CPU (General, Compute, Math & Sorting)

    af-alg

    bitonicsort

    bsearch

    bubblesort

    cpu

    dfp

    fibsearch

    fma

    fp

    hash

    heapsort

    hsearch

    insertionsort

    jpeg

    judy

    lsearch

    matrix

    matrix-3d

    mergesort

    qsort

    radixsort

    randlist

    rotate

    shellsort

    str

    tree

    tsc

    tsearch

    vecfp

    vecwide

    wcs

    zlib

🚦 CPU (Cache, Atomics & Mutex)

    l1cache

    prefetch

💾 Memory (RAM)

    bigheap

    malloc

    memcpy

    stream - Better throughput with less stressors

📄 Virtual Memory (VM) & Paging

    mmap

    mremap

    pageswap

    vm

    vm-rw

💽 Storage I/O (Disk)

    copy-file

    fallocate

    fpunch

    hdd

    readahead

    seek

    urandom

📁 Filesystem (Metadata & Ops)

    dentry

    inotify

    metamix

    sysfs

    tmpfs

    utime

🌐 Network

    poll

    sockpair

🏭 Kernel (Process, Scheduling & Unsharing)

    affinity

    exec

    fork

    spawn

    vfork

    yield

    zombie

🔔 Kernel (Signals)

    kill

    sigbus

    sigfd

    sigfpe

    sigill

    sigsegv

📨 Kernel (IPC, Pipes & Semaphores)

    futex

    mq

    pipe

⏰ Kernel (Timers & Clocks)

    alarm

    clock

🛂 Kernel (Syscalls & Security)

    env

    get

    kcmp

    sysinfo




Avoid these?
 🚫 Verification Not Implemented

These stressors do not currently have a verification method.

🧠 CPU (General, Compute, Math & Sorting)

    easy-opcode

    fractal

    ipsec-mb

    monte-carlo

    nop

    opcode

    plugin

    prime

    regex

🚦 CPU (Cache, Atomics & Mutex)

    cache

    flipflop

    flushcache

    icache

    lockbus

    membarrier

💾 Memory (RAM)

    mcontend

    memhotplug

    memrate

    memthrash

    physmmap

    pkey

    secretmem

    spinmem

📄 Virtual Memory (VM) & Paging

    fault

    idle-page

    madvise

    mlockmany

    mmapcow

    mmaprandom

    mmaptorture

    tlb-shootdown

    vma

💽 Storage I/O (Disk)

    dev

    splice

📁 Filesystem (Metadata & Ops)

    fd-abuse

    filerace

    inode-flags

    open

    procfs

    unlink

🌐 Network

    netlink-proc

    ping-sock

🏭 Kernel (Process, Scheduling & Unsharing)

    forkheavy

    loadavg

    resources

    syncload

🔔 Kernel (Signals)

    sigurg

📨 Kernel (IPC, Pipes & Semaphores)

    ring-pipe

⏰ Kernel (Timers & Clocks)

    cyclic

🛂 Kernel (Syscalls & Security)

    bad-ioctl

    dynlib

    enosys

    lsm

    module

    rseq

    sysbadaddr

    syscall

    sysinval

    uprobe

    vdso

🔌 I/O & Peripherals

    gpu

    led

    pci
