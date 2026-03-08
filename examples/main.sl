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
    a = 0
    b = 1
    iter = 1
    while iter < n {
        c = b
        b = a + b
        a = c
        iter = iter + 1
    }

    return b
}

x = 0
while x < 500 {
    print fib_dyn(x)
    x = x + 1
}
