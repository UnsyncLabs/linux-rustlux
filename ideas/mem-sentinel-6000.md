# Mem Sentinel v2 — mejoras si memtest86+ pasa 24 h a 6000 MT/s

Escenario: memtest86+ corre un día entero a 6000 MT/s sin un solo error,
pero Linux crashea a esa frecuencia. Conclusión: la RAM es estable bajo
carga continua — la inestabilidad la disparan comportamientos que solo
un OS produce: transiciones de power management, idle profundo,
power-down de la DRAM. memtest nunca deja la máquina en idle, así que
nunca pisa ese escenario.

Estas son mejoras teóricas de kernel para ese caso. Ninguna "arregla"
la RAM (eso sigue siendo firmware/BIOS); todas detectan, correlacionan
o esquivan el escenario frágil.

## 1. Correlación error ↔ estado de energía (diagnóstico)

Hoy el sentinel solo cuenta errores. Mejora: al registrar cada MCE,
capturar también el contexto de energía del momento:

- residencia reciente de C-states profundos (CC6/PC6) por CPU
  (`cpuidle` ya expone `usage`/`time` por estado)
- si el error llegó estando idle (`tick_nohz_tick_stopped()`)
- timestamp relativo al último wakeup

Log resultante: `rustlux_memsentinel: error #N — 87% del último
segundo en CC6, error a 2ms del wakeup`. Eso convierte "crashea a
6000" en "crashea al salir de idle profundo", que es accionable
(desactivar Power Down Enable / Memory Context Restore en BIOS).

## 2. Limitador de idle adaptativo (mitigación automática)

Si la correlación del punto 1 confirma que los errores llegan tras
idle profundo, el sentinel puede esquivar el escenario frágil en
runtime sin tocar el BIOS:

- al cruzar el umbral de errores correlacionados con idle, registrar
  un `cpu_latency_qos_add_request()` (PM QoS) que impida entrar a
  C-states profundos — el mismo mecanismo que usan los drivers de
  baja latencia
- mantenerlo N minutos; si no reaparecen errores, relajar
  gradualmente (backoff exponencial)
- sysctl: `vm.rustlux_memsentinel_idle_guard` (0/1) +
  cuenta de activaciones visible en `/proc`

Costo: más consumo en idle mientras está activo. Beneficio: la
máquina deja de crashear *sola*, y el log dice exactamente por qué.

## 3. Páginas canario (detector para hardware sin ECC visible)

En consumer DDR5 sin ECC de host, un bit flip en datos no genera MCE —
solo los errores del controlador se ven. Mejora: detector activo de
corrupción silenciosa:

- reservar N páginas (config, p.ej. 64 × 4 KiB) repartidas por zonas
  altas y bajas de memoria física, escritas con patrones conocidos
  (0xAA55..., walking bits)
- un kworker las verifica cada M segundos, con prioridad a la
  verificación justo después de salir de suspend/idle largo
  (donde el punto 1 dice que está el riesgo)
- un canario corrupto = evidencia dura de bit flip → mismo pipeline
  del sentinel (warn + taint), con dirección física exacta del fallo

Es la versión en runtime de memtest: barata (KiB, no GiB), continua,
y corre exactamente en las condiciones reales del OS que memtest no
reproduce. Core de verificación en Rust (`rustlux_mm`), testeable
standalone como los demás crates.

## 4. Scrub agresivo bajo sospecha (EDAC)

Los controladores AMD/Intel tienen patrol scrub configurable vía EDAC
(`/sys/devices/system/edac/mc/mc*/sdram_scrub_rate`). Mejora: cuando
el sentinel entra en estado WARN, subir temporalmente el scrub rate al
máximo para que los errores latentes se corrijan/detecten antes de que
se acumulen en la misma palabra ECC. Volver al rate normal al salir
del estado de sospecha.

## 5. Verificación de memoria al boot dirigida

`memtest=N` ya existe pero prueba patrones simples y alarga mucho el
boot si se usa completo. Mejora: modo `rustlux_memtest=fast` que
pruebe solo las regiones estadísticamente problemáticas (fin de cada
rank, bordes de canal — derivables del layout físico) con los patrones
que más estresan DDR5 (hammer de filas adyacentes, transiciones
simultáneas de bus). 2-3 s de boot en vez de minutos.

## Plan de prueba (orden)

1. memtest86+ desde USB @ 6000, 24 h → si falla, es plataforma: fin.
2. Si pasa: Linux @ 6000 con `rasdaemon` instalado (o el sentinel una
   vez buildeado) y anotar si los crashes/errores correlacionan con
   idle. `journalctl -k | grep -i "mce\|machine check"` tras cada
   sesión.
3. Probar en BIOS: `Power Down Enable = off`,
   `Memory Context Restore = off` → si desaparecen los crashes,
   implementar los puntos 1 y 2 primero (son los que atacan esa causa).
4. Si los crashes ocurren también bajo carga en Linux pero no en
   memtest: sospechar del stack gráfico/PCIe (VRAM ↔ RAM), no de la
   DRAM — otro problema distinto.
