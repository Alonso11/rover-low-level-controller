// Version: v1.0
// Archivo para Pruebas Unitarias de Lógica (Unit Tests)
// Estas pruebas se ejecutan en tu PC, no en el Arduino.
// Comando para correrlas: cargo test --target x86_64-unknown-linux-gnu (o tu arquitectura de PC)

#[cfg(test)]
mod tests {
    #[test]
    fn test_example_logic() {
        // Aquí podrías probar, por ejemplo, que el mapeo de -100..100 a 0..255 es correcto
        let speed: i16 = -50;
        let abs_speed = speed.abs();
        
        assert_eq!(abs_speed, 50);
        assert!(speed < 0, "La velocidad debería indicar retroceso");
    }

    // Nota: Para probar el struct L298NMotor aquí, 
    // necesitaríamos mover la lógica a un archivo src/lib.rs
}
