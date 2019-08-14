function int f(int a);
    return a * a;
endfunction

function int g(int a, int b);
    return f(a, b) + b;
endfunction

function int h(int long_name_a, int long_name_b, int long_name_c, int long_name_d);
    return 0;
endfunction
