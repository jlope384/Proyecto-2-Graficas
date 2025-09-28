// ========== GUÍA DE CONTROLES OPTIMIZADA ==========

/* CONTROLES DE CÁMARA:
 * ═════════════════════════════════════════════════════════════
 * Flechas:        Órbita de cámara (rotación alrededor del centro)
 * W / S:          Zoom in / Zoom out
 * Shift + W/S:    Zoom rápido (3x velocidad)
 * Page Up/Down:   Ajustar velocidad de zoom
 */

/* CONTROLES DE ROTACIÓN DE ESCENA:
 * ═════════════════════════════════════════════════════════════
 * Q / E:          Acelerar/desacelerar rotación automática
 * A / D:          Rotación manual (izquierda/derecha)
 * Shift + A/D:    Rotación manual rápida
 * Space:          Detener rotación automática
 * R:              Reset (detener y centrar rotación)
 */

/* CONTROLES DE SKYBOX:
 * ═════════════════════════════════════════════════════════════
 * 1:              Atardecer atmosférico
 * 2:              Mediodía soleado
 * 3:              Noche estrellada
 * 4:              Cielo nublado
 * 5:              Espacio cósmico
 */

/* OPTIMIZACIONES IMPLEMENTADAS:
 * ═════════════════════════════════════════════════════════════
 * ✅ Eliminación de código muerto y variables no utilizadas
 * ✅ Pre-reserva de memoria para vectores (Vec::with_capacity)
 * ✅ Detección única de modificadores (Shift)
 * ✅ Uso de else-if para controles mutuamente exclusivos
 * ✅ Clamp optimizado con min/max
 * ✅ Creación condicional de objetos rotados (solo cuando necesario)
 * ✅ Eliminación de verificaciones redundantes
 * ✅ Organización lógica de controles por categoría
 */

/* CARACTERÍSTICAS DE RENDIMIENTO:
 * ═════════════════════════════════════════════════════════════
 * - Rotación de escena sin copias innecesarias cuando angle = 0
 * - Matriz de rotación calculada una sola vez por frame
 * - Controles optimizados con lógica simplificada
 * - Variables de velocidad con límites seguros
 * - Zoom con multiplicador dinámico según modificadores
 */

/* FLUJO DE RENDERIZADO OPTIMIZADO:
 * ═════════════════════════════════════════════════════════════
 * 1. Verificar cambios de cámara/escena
 * 2. Aplicar rotación global solo si es necesaria
 * 3. Usar sistema LOD adaptativo para mejor rendimiento
 * 4. Renderizado progresivo para calidad máxima
 * 5. Sistema de caché de framebuffer optimizado
 */

/* USO RECOMENDADO:
 * ═════════════════════════════════════════════════════════════
 * 1. Inicia con skybox #1 (atardecer) para mejores efectos
 * 2. Usa Q para comenzar rotación automática lenta
 * 3. Experimenta con zoom (W/S) para diferentes perspectivas
 * 4. Prueba skyboxes diferentes (#2-5) con rotación activa
 * 5. Usa R para reset cuando quieras centrar la vista
 */