function int f(int a);
    return a * a;
endfunction

function int g(int a, int b);
    return f(a, b) + b;
endfunction
