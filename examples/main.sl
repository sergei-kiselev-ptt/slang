func fib_recur(n: num) -> num {
    if n <= 1 {
        return n
    }
    return fib_recur(n - 1) + fib_recur(n - 2)
}

func fib_dyn(n: num) -> num {
    if n <= 1 {
        return n
    }
    let a: num = 0
    let b: num = 1
    let iter: num = 1
    while iter < n {
        let c: num = b
        b = a + b
        a = c
        iter = iter + 1
    }

    return b
}

let x: num = 1
while x < 50 {
    print fib_dyn(x)
    x = x + 1
}
