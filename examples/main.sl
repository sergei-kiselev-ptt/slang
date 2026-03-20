func fib_recur(n: int) -> int {
    if n <= 1 {
        return n
    }
    return fib_recur(n - 1) + fib_recur(n - 2)
}

func fib_dyn(n: int) -> int {
    if n <= 1 {
        return n
    }
    let mut a: int = 0
    let mut b: int = 1
    let mut iter: int = 1
    while iter < n {
        let c: int = b
        b = a + b
        a = c
        iter = iter + 1
    }

    return b
}



let x: int = 1

for i in 1..11 {
    for i in 1..11 {
        print i*j
    }
}
