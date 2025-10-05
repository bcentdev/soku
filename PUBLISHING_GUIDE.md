# Soku Publishing Guide

Esta guía te muestra cómo publicar Soku (速) en npm de forma fácil y automatizada.

## 🚀 Publicación Automática (Recomendado)

Soku está configurado para publicarse automáticamente en npm cuando creas un release en GitHub.

### Requisitos Previos

1. **Token npm**: Genera un token en npmjs.com
   - Ve a https://www.npmjs.com/settings/{tu-usuario}/tokens
   - Create New Token → Classic Token
   - Type: Automation (recomendado para CI/CD)
   - Copia el token

2. **Añadir token a GitHub**:
   - Ve a tu repo → Settings → Secrets and variables → Actions
   - New repository secret
   - Name: `NPM_TOKEN`
   - Value: tu token npm

### Proceso Automático

```bash
# 1. Actualiza versión en ambos archivos
# Edita Cargo.toml:
version = "0.3.1"

# Edita package.json:
"version": "0.3.1"

# 2. Actualiza CHANGELOG.md
# Añade sección con cambios de esta versión:
## [0.3.1] - 2025-01-XX
### Added
- Nueva funcionalidad...

# 3. Commit cambios
git add Cargo.toml package.json CHANGELOG.md
git commit -m "chore: bump version to 0.3.1"
git push

# 4. Crea release automáticamente
./scripts/create-release.sh

# O manualmente:
git tag -a v0.3.1 -m "Release 0.3.1"
git push origin v0.3.1
```

**¡Listo!** GitHub Actions:
1. ✅ Compila binarios para todas las plataformas
2. ✅ Crea el release en GitHub
3. ✅ Publica todos los paquetes en npm
4. ✅ Actualiza el release con link de npm

Esto toma ~15-20 minutos. Verás el progreso en GitHub Actions.

---

## 📦 Publicación Manual (Opcional)

Si prefieres publicar manualmente o las GitHub Actions fallan:

### Paso 1: Login a npm

```bash
npm login
# Username: tu-usuario
# Password: tu-password (o token)
# Email: tu-email
```

Verifica:
```bash
npm whoami
# Debe mostrar tu usuario
```

### Paso 2: Compilar Binarios

Necesitas compilar para todas las plataformas:

```bash
# macOS ARM64 (M1/M2/M3)
cargo build --release --target aarch64-apple-darwin

# macOS Intel
cargo build --release --target x86_64-apple-darwin

# Linux x64
cargo build --release --target x86_64-unknown-linux-gnu

# Linux ARM64 (requiere cross-compilation)
cargo build --release --target aarch64-unknown-linux-gnu

# Windows x64 (requiere Windows o cross-compilation)
cargo build --release --target x86_64-pc-windows-msvc
```

**Nota**: Si no puedes compilar para todas las plataformas localmente, usa las GitHub Actions.

### Paso 3: Probar Localmente (Opcional)

```bash
# Probar que los paquetes se crean correctamente
./scripts/prepare-npm-packages.sh

# Hacer dry-run (no publica realmente)
./scripts/publish-npm.sh dry-run
```

### Paso 4: Publicar

```bash
# Publica todos los paquetes a npm
./scripts/publish-npm.sh
```

Esto:
1. Verifica que estás logueado
2. Verifica que todos los binarios existen
3. Crea paquetes de plataforma
4. Publica cada paquete de plataforma
5. Publica el paquete principal

---

## 🏷️ Crear Release Sin Publicar

Si solo quieres crear el tag y release pero publicar después:

```bash
# Crear tag localmente
git tag -a v0.3.1 -m "Release 0.3.1"

# Pushear tag
git push origin v0.3.1

# GitHub Actions compilará binarios y creará release
# Pero NO publicará a npm si el secret NPM_TOKEN no existe
```

Después puedes publicar manualmente cuando quieras con `./scripts/publish-npm.sh`.

---

## 🔧 Scripts Disponibles

| Script | Descripción |
|--------|-------------|
| `./scripts/create-release.sh` | Crea tag y release automáticamente |
| `./scripts/prepare-npm-packages.sh` | Prepara paquetes npm (no publica) |
| `./scripts/publish-npm.sh` | Publica a npm |
| `./scripts/publish-npm.sh dry-run` | Simula publicación (no publica) |

---

## ✅ Verificar Publicación

Después de publicar, verifica:

```bash
# Ver en npm registry
npm view soku

# Instalar y probar
npm install -g soku
soku --version
soku build --help
```

También puedes ver en:
- https://www.npmjs.com/package/soku
- https://www.npmjs.com/package/soku-darwin-arm64
- https://www.npmjs.com/package/soku-darwin-x64
- https://www.npmjs.com/package/soku-linux-x64
- https://www.npmjs.com/package/soku-linux-arm64
- https://www.npmjs.com/package/soku-win32-x64

---

## 🐛 Troubleshooting

### "Not logged in to npm"
```bash
npm login
npm whoami  # Verificar
```

### "Version already published"
- Incrementa la versión en `Cargo.toml` y `package.json`
- No puedes sobrescribir versiones en npm

### "Binary not found"
- Compila todos los binarios primero
- O usa GitHub Actions para compilar automáticamente

### GitHub Actions falla en npm publish
- Verifica que `NPM_TOKEN` está en Secrets
- El token debe tener permisos de `Automation`
- Regenera el token si ha expirado

### "Permission denied" al publicar
- Verifica que tu usuario npm tiene permisos en el scope
- Para paquetes sin scope, debes ser owner/maintainer

---

## 📝 Checklist de Release

- [ ] Versión actualizada en `Cargo.toml`
- [ ] Versión actualizada en `package.json`
- [ ] `CHANGELOG.md` actualizado con cambios
- [ ] Tests pasando: `cargo test`
- [ ] Build exitoso: `cargo build --release`
- [ ] Cambios commiteados y pusheados
- [ ] Token npm configurado en GitHub Secrets
- [ ] Tag creado: `./scripts/create-release.sh`
- [ ] GitHub Actions completado exitosamente
- [ ] Verificado en npmjs.com

---

## 🎯 Resumen

**Proceso Normal (Automático)**:
1. Actualiza versiones y changelog
2. Commit y push
3. Ejecuta `./scripts/create-release.sh`
4. Espera ~15 minutos
5. ✅ Publicado en npm automáticamente

**Proceso Manual** (si es necesario):
1. Compila binarios para todas las plataformas
2. Ejecuta `./scripts/publish-npm.sh`
3. ✅ Publicado en npm

¡Eso es todo! 🚀
