// Programa de prueba: imprime un mensaje, calcula 6*7 con un bucle y sale con ese código.
.text
_start:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    adr x0, msg
    mov x1, #14
    svc #0x27
    mov x2, #0          // acumulador
    mov x3, #6          // contador
loop:
    add x2, x2, #7
    subs x3, x3, #1
    b.ne loop
    mov x4, #1
    lsl x4, x4, #4      // 16
    udiv x5, x2, x4     // 42/16 = 2
    mul x6, x5, x4      // 32
    sub x0, x2, x6      // 10
    add x0, x0, x5      // 12
    bl helper
    ldp x29, x30, [sp], #16
    svc #0x7
helper:
    add x0, x0, #30     // 42
    ret
msg:
    .ascii "Hola, Switch!\n"
