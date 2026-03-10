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
    let mut a: num = 0
    let mut b: num = 1
    let mut iter: num = 1
    while iter < n {
        let c: num = b
        b = a + b
        a = c
        iter = iter + 1
    }

    return b
}



let x: num = 1

if x == 1 {
    print 1
    x + 2
} else {
    print 999
}

print 100
