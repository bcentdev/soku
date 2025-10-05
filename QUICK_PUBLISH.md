# 🚀 Guía Rápida de Publicación

## Configuración Inicial (Solo una vez)

### 1. Crear Token npm
1. Ve a https://www.npmjs.com/settings/{tu-usuario}/tokens
2. "Generate New Token" → Classic Token
3. Type: **Automation**
4. Copia el token

### 2. Añadir a GitHub
1. GitHub repo → Settings → Secrets and variables → Actions
2. "New repository secret"
3. Name: `NPM_TOKEN`
4. Value: pega tu token

## 📦 Publicar Nueva Versión (Automático)

```bash
# 1. Actualiza versiones
# Edita Cargo.toml: version = "0.3.1"
# Edita package.json: "version": "0.3.1"

# 2. Actualiza CHANGELOG.md
# Añade:
## [0.3.1] - 2025-01-XX
### Added
- Nueva funcionalidad

# 3. Commit
git add Cargo.toml package.json CHANGELOG.md
git commit -m "chore: bump version to 0.3.1"
git push

# 4. Crea release
./scripts/create-release.sh
```

**¡Listo!** En ~15 minutos Ultra estará en npm.

Verifica en: https://www.npmjs.com/package/ultra-bundler

## 🔧 Publicar Manualmente (Si necesario)

```bash
# 1. Login
npm login

# 2. Publica
./scripts/publish-npm.sh
```

## 📝 Archivos Importantes

| Archivo | Qué hace |
|---------|----------|
| `scripts/create-release.sh` | Crea tag y release |
| `scripts/publish-npm.sh` | Publica a npm |
| `PUBLISHING_GUIDE.md` | Guía completa |

## ⚡ Test Rápido

```bash
# Probar sin publicar
./scripts/publish-npm.sh dry-run

# Instalar tu versión
npm install -g ultra-bundler
ultra --version
```

---

Ver [PUBLISHING_GUIDE.md](./PUBLISHING_GUIDE.md) para detalles completos.
