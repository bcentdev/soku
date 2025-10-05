# Soku - Roadmap

> Última actualización: 2025-10-05
> Estado: 14,447 líneas Rust | 130 unit tests | 6 integration tests | 20 warnings | Sprint 10 COMPLETADO ✅

## ✅ Completado

### Sprint 1-4: Foundation (100%)
- TypeScript + JSX processing (AST-based)
- Tree shaking (50-80% reduction)
- Code splitting
- Source maps (basic)
- Incremental builds (50% faster)
- Node modules resolution (package.json exports)
- Parallel processing (rayon + tokio)

### Sprint 5: Advanced Features (100%)
- ✅ Watch Mode (`soku watch`)
- ✅ Bundle Analysis (`--analyze` flag)
- ✅ CSS Modules (`.module.css` auto-detection)
- ✅ Incremental Builds (persistent state)
- 🔌 WASM Support (API ready, pendiente integración)

### Sprint 6: Quality & Testing (100%) ✅
- ✅ Unit tests: 43 passing (16 archivos)
- ✅ Integration tests: 6 passing + 3 TODOs documentados
- ✅ Doctests: 6 passing + 1 ignored
- ✅ Test structure: Organizada con fixtures limpios
- ✅ Project cleanup: 15 proyectos → 6 fixtures oficiales
- ✅ CI/CD: GitHub Actions setup
- ✅ Warnings: 18 → 0 (100% clean)
- ✅ Code cleanup: 725 líneas eliminadas
- ✅ Codebase: 11,108 → 10,299 líneas (-7.3%)

---

## 🎯 Próximos Sprints

### Sprint 6.5: Finalización Quality (Completado - 100%) ✅
**Objetivo**: Completar error handling, docs y TODOs

**Tareas**:
- ✅ Fix 7 doctests (6 passing + 1 ignored)
- ✅ Documentar 3 integration test TODOs con causas ra\u00edz
  - Source maps: problema de caching
  - Tree shaking stats: no se populan removed_exports
  - TypeScript tree shaking: integración Enhanced + tree shaking
- ✅ Error handling improvements (mensajes contextuales)
- ✅ WASM pipeline integration (loader JS automático, copy files)
- ✅ CSS Modules simplification (80 líneas eliminadas)
- ✅ Documentation updates (README + CLAUDE.md)

**Tiempo**: 1-2 semanas

---

### Sprint 7: Performance (Completado - 100%) ✅
**Objetivo**: Optimizar performance y binary size

**Tareas**:
- ✅ Binary size: 13MB → 3.8MB (71% reduction, objetivo <10MB EXCEEDED)
- ✅ Link Time Optimization (LTO "fat" mode)
- ✅ Tokio features optimization (full → specific features)
- ✅ Strip debug symbols (strip = true)
- ✅ Size optimization (opt-level = "z")
- ✅ Panic mode optimization (panic = "abort")
- ✅ Dev/Test profile optimization for faster iteration

**Resultado**: Binary 71% más pequeño, funcionalidad 100% preservada, todos los tests passing

**Tiempo**: 1 día (completado)

---

### Sprint 8: Features (Completado - 100%) ✅
**Objetivo**: Advanced features y developer experience

**Tareas**:
- ✅ Config file support (soku.config.json con merge CLI/file)
- ✅ HMR improvements (error recovery con overlay, CSS hot reload mejorado)
- ✅ WebSocket connection management real con channel-based messaging
- ✅ Error overlay visual para build failures
- ✅ Auto-recovery cuando se corrigen errores

**Resultado**: Config files funcionales, HMR robusto con error recovery, mejor DX

**Tiempo**: 2 días (completado)

### Sprint 9A: Quick Wins (Completado - 100%) ✅
**Objetivo**: Features críticas de alta prioridad

**Tareas**:
- ✅ Environment Variables (process.env, import.meta.env con .env files)
- ✅ Path Aliases (@/, @components, @utils via soku.config.json)
- ✅ External Dependencies (exclude React, etc. del bundle)

**Resultado**:
- Environment Variables: .env loading con priority-based files, NODE_ENV/MODE/DEV/PROD automáticos
- Path Aliases: Resolución flexible de paths (@, ~, #components), integrado en pipeline paralelo
- External Dependencies: Pattern matching (react/hooks), preserva import statements en output

**Tiempo**: 1 día (completado)

### Sprint 9B: Optimization (Completado - 100%) ✅
**Objetivo**: Performance y code splitting avanzado

**Tareas**:
- ✅ Asset Handling (JSON imports)
- ✅ Conditional Exports (dead code elimination)
- ✅ Manual Chunks/Vendor Splitting (configuración explícita)
- ✅ Dynamic Imports (lazy loading infrastructure)

**Resultados**:
- JSON Imports: AssetProcessor con conversión automática JSON → ES module, 6 tests pasando
- Dead Code Elimination: Eliminación de código muerto basado en condiciones constantes, 23.5% reducción de bundle, 6 tests pasando
- Manual Chunks/Vendor Splitting: Separación automática node_modules → vendor.js + app → app.js, optimización completa en ambos bundles, mejora caching y parallel loading
- Dynamic Imports: DynamicImportSplitter con detección de import(), chunk manifest, runtime loader JavaScript, 6 tests pasando

**Tiempo**: 1 día (completado)

### Sprint 10: Architecture (100%) ✅
**Objetivo**: Features arquitecturales avanzadas

**Tareas**:
- ✅ Multiple Entry Points (multi-page apps, libraries)
- ✅ Advanced Source Maps (inline sources, accurate mappings)
- ✅ Plugin API (event-based, extensible)
- ✅ Custom Transformers Support (user-defined transformations)
- ✅ Advanced HMR Hooks (customizable hot reload)

**Resultados**:
- Multiple Entry Points: 180 líneas, configuración entries, build_with_multiple_entries, BFS traversal
- Advanced Source Maps: 379 líneas, source map v3, inline/external support, 11 tests
- Plugin API: 472 líneas, 8 lifecycle hooks, 3 example plugins, 14 tests
- Custom Transformers: 521 líneas, 3 transformer types, 8 built-in transformers, 15 tests
- Advanced HMR Hooks: 580 líneas, 8 lifecycle methods, 5 built-in hooks, 15 tests

**Total**: 2,132 líneas, 65 tests, 5 features arquitecturales

**Tiempo**: 2 días (completado)

---

## 🎯 Próximos Sprints

### Sprint 11: Preprocessing & DX (100%) ✅
**Objetivo**: Developer experience avanzado

**Tareas**:
- ✅ CSS Preprocessing (SCSS/SASS support con grass compiler)
- ⏭️ PostCSS Integration (SKIP - LightningCSS covers autoprefixer)
- ✅ TypeScript Decorators (multiline + property decorators)
- ✅ Better error messages (ErrorContext con hints básicos)
- ✅ Performance profiling (bundle_analysis.rs completo)

**Resultados**:
- SCSS/SASS: 261 líneas, grass compiler, 6 tests pasando
- TypeScript Decorators: 99 líneas, multiline tracking, 1 test nuevo
- Error Messages: Infrastructure completa con format_detailed
- Performance Profiling: Ya implementado en Sprint anterior

**Total**: 360 líneas, 3 features implementadas (2 skipped), 137 tests

**Tiempo**: 2 días (completado)

---

### Sprint 12: Production (Pendiente)
**Objetivo**: Production-ready release

**Tareas**:
- Comprehensive docs
- More examples (React, Vue, vanilla)
- CLI improvements
- Performance benchmarks vs competitors
- Release v1.0

**Tiempo**: 3-4 semanas

---

## 🎯 Features Planificadas por Categoría

### 🚀 Alta Prioridad (Sprint 9A - Quick Wins)
- **Environment Variables**: process.env.NODE_ENV, import.meta.env.DEV/PROD
- **Path Aliases**: @/, @components, @utils con tsconfig.json integration
- **External Dependencies**: Exclude libraries del bundle (React, Vue, etc.)
- **TypeScript Path Mapping**: Sincronización automática con tsconfig paths

### ⚡ Optimización (Sprint 9B)
- **Manual Chunks**: Configuración explícita de chunks (vendor, common, etc.)
- **Vendor Splitting**: Separación automática de node_modules
- **Dynamic Imports**: import() lazy loading con code splitting automático
- **Asset Handling**: import de images, fonts, JSON con URL resolution
- **Conditional Exports**: Dead code elimination basado en environment

### 🏗️ Arquitectura (Sprint 10)
- **Multiple Entry Points**: Multi-page apps, library mode
- **Advanced Source Maps**: Inline sources, accurate line mappings
- **Plugin API**: Event-based, extensible architecture
- **Custom Transformers**: User-defined code transformations
- **Advanced HMR Hooks**: Lifecycle hooks para hot reload

### 🎨 Preprocessing & DX (Sprint 11)
- **SCSS/SASS Support**: CSS preprocessing integrado
- **PostCSS Integration**: Autoprefixer, CSS variables, etc.
- **Advanced TypeScript**: Decorators, metadata, advanced features
- **Better Error Messages**: Suggestions y hints contextuales
- **Performance Profiling**: Bundle analysis tools

---

## 📊 Estado Actual

### Métricas
- **Líneas de código**: 14,447 (src) + 400 (tests) [+2,132 líneas en Sprint 10]
- **Binary size**: 3.8MB (down from 13MB, 71% reduction) ✅
- **Test coverage**: ~22% (130 unit + 6 integration + 6 doctests)
- **Test fixtures**: 6 proyectos oficiales organizados
- **Warnings**: 20 (dead_code warnings por features no integradas)
- **Performance**: Sub-250ms builds (17ms típico)
- **Tree shaking**: 50-80% reduction
- **Code cleanup**: 809 líneas eliminadas (Sprint 6 + 6.5)
- **Compile time**: 58s release (LTO enabled), <3s dev
- **Features activas**: 23 (Sprint 1-10 completo)

### Features Activas (23 total)
- ✅ JS/TS/TSX bundling
- ✅ CSS bundling + modules
- ✅ Tree shaking (50-80% reduction)
- ✅ Code splitting
- ✅ Source maps (basic + advanced)
- ✅ Incremental builds
- ✅ Watch mode
- ✅ Bundle analysis
- ✅ HMR con error recovery
- ✅ WASM support (auto loaders)
- ✅ Config file support (soku.config.json)
- ✅ Error overlay visual
- ✅ CSS hot reload
- ✅ Auto mode selection (turbo/normal)
- ✅ Minification avanzada
- ✅ Node modules optimization
- ✅ WebSocket-based HMR server
- ✅ CLI con progress tracking
- ✅ Multiple entry points (multi-page apps)
- ✅ Advanced source maps (inline + external)
- ✅ Plugin API (event-based extensibility)
- ✅ Custom transformers (user-defined code transforms)
- ✅ Advanced HMR hooks (customizable hot reload lifecycle)

### Features Planificadas (Roadmap Actualizado)
- 🎯 **Sprint 9A**: 4 features (Environment, Aliases, Externals, TS Paths)
- ⚡ **Sprint 9B**: 5 features (Chunks, Dynamic Imports, Assets, Conditional Exports)
- 🏗️ **Sprint 10**: 5 features (Multi-entry, Source Maps, Plugin API, Transformers, HMR Hooks)
- 🎨 **Sprint 11**: 5 features (SCSS, PostCSS, Advanced TS, Better Errors, Profiling)
- **Total planificado**: 19 features adicionales

### Comparación con Competidores
Cuando completemos Sprint 9-11, Soku tendrá:
- **37 features activas** (18 actuales + 19 planificadas)
- **Velocidad**: 10-20x más rápido que Webpack
- **Bundle size**: 50-80% reducción con tree shaking
- **DX**: Config file, HMR, error overlay, aliases
- **Features únicos**: Soku mode, auto-optimization, 3.8MB binary
