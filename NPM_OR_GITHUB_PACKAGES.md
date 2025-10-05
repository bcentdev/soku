# Publishing Ultra: npm vs GitHub Packages

Ultra puede publicarse en **dos registros diferentes**. Esta guía te ayuda a elegir el mejor para ti.

## 📊 Comparación Rápida

| Característica | npmjs.org | GitHub Packages |
|----------------|-----------|-----------------|
| **Visibilidad** | Público por defecto | Privado por defecto (puede ser público) |
| **Autenticación** | Token npm | Token GitHub (PAT) |
| **Costo** | Gratis para públicos | Gratis (límites según plan) |
| **Nombre del paquete** | `ultra-bundler` | `@bcentdev/ultra` |
| **Descubrimiento** | ⭐⭐⭐⭐⭐ Muy alto | ⭐⭐⭐ Moderado |
| **Integración GitHub** | No integrado | ✅ Totalmente integrado |
| **Facilidad instalación** | `npm install ultra-bundler` | Requiere `.npmrc` extra |

## 🎯 ¿Cuál elegir?

### Elige **npmjs.org** si:
- ✅ Quieres máxima visibilidad y descubrimiento
- ✅ Quieres instalación más simple para usuarios
- ✅ Es tu primer paquete público
- ✅ Quieres aparecer en búsquedas npm

### Elige **GitHub Packages** si:
- ✅ Quieres mantener todo en GitHub
- ✅ Ya tienes organización en GitHub
- ✅ Quieres aprovechar permisos de GitHub
- ✅ Planeas paquetes privados futuros

### ¿Por qué no ambos? 🤷‍♂️
Puedes publicar en ambos, pero:
- Requiere mantener dos configuraciones
- Los nombres deben ser diferentes
- Más complejo para usuarios (confusión sobre dónde instalarlo)

**Recomendación**: Empieza con **npmjs.org** para máximo alcance.

---

## 📦 Opción A: Publicar en npmjs.org

### 1. Configuración (Ya está lista)

El `package.json` actual ya está configurado para npmjs.org:

```json
{
  "name": "ultra-bundler",
  "optionalDependencies": {
    "@ultra-bundler/darwin-arm64": "0.3.0",
    "@ultra-bundler/darwin-x64": "0.3.0",
    // ...
  }
}
```

### 2. Autenticación

```bash
# Login a npm
npm login

# Verificar login
npm whoami
```

### 3. Publicar

```bash
# 1. Preparar paquetes de plataforma
./scripts/prepare-npm-packages.sh

# 2. Publicar cada paquete de plataforma
cd npm-packages/darwin-arm64
npm publish --access public

# Repetir para cada plataforma...

# 3. Publicar paquete principal
cd ../..
npm publish --access public
```

### 4. Instalación para usuarios

```bash
npm install -g ultra-bundler
yarn global add ultra-bundler
pnpm add -g ultra-bundler

# O sin instalar
npx ultra-bundler build
```

---

## 🐙 Opción B: Publicar en GitHub Packages

### 1. Cambiar configuración

```bash
# Usar el package.json para GitHub
cp package.github.json package.json

# Copiar configuración de npm
cp .npmrc.github .npmrc
```

Tu `package.json` ahora tiene:

```json
{
  "name": "@bcentdev/ultra",
  "publishConfig": {
    "registry": "https://npm.pkg.github.com"
  },
  "optionalDependencies": {
    "@bcentdev/ultra-darwin-arm64": "0.3.0",
    // ...
  }
}
```

### 2. Crear Personal Access Token (PAT)

1. Ve a GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Generate new token (classic)
3. Scopes necesarios:
   - ✅ `write:packages` (publicar paquetes)
   - ✅ `read:packages` (leer paquetes)
   - ✅ `delete:packages` (opcional, eliminar versiones)
4. Copia el token

### 3. Autenticación

**Opción A: Variable de entorno**
```bash
export GITHUB_TOKEN=ghp_tu_token_aqui
```

**Opción B: Editar .npmrc**
```bash
# .npmrc
@bcentdev:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=ghp_tu_token_aqui
```

**Opción C: npm login**
```bash
npm login --scope=@bcentdev --registry=https://npm.pkg.github.com
# Username: tu-username-github
# Password: tu-PAT-token
# Email: tu-email
```

### 4. Publicar

```bash
# 1. Preparar paquetes (detecta automáticamente el scope)
./scripts/prepare-npm-packages.sh

# 2. Publicar cada paquete de plataforma
cd npm-packages/darwin-arm64
npm publish

# Repetir para cada plataforma...

# 3. Publicar paquete principal
cd ../..
npm publish
```

### 5. Instalación para usuarios

Los usuarios necesitan configurar `.npmrc` en su proyecto o globalmente:

```bash
# En el proyecto del usuario
echo "@bcentdev:registry=https://npm.pkg.github.com" > .npmrc

# Luego instalar
npm install -g @bcentdev/ultra

# O para proyectos privados, necesitan autenticación
echo "//npm.pkg.github.com/:_authToken=\${GITHUB_TOKEN}" >> .npmrc
```

---

## 🤖 Automatización con GitHub Actions

### Para npmjs.org

Añade `NPM_TOKEN` a GitHub Secrets:
1. Genera token en npmjs.com → Access Tokens → Generate New Token
2. GitHub repo → Settings → Secrets → New repository secret
3. Nombre: `NPM_TOKEN`, Valor: tu token npm

```yaml
# .github/workflows/publish-npm.yml
- name: Publish to npm
  run: npm publish --access public
  env:
    NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

### Para GitHub Packages

Usa el `GITHUB_TOKEN` automático:

```yaml
# .github/workflows/publish-github.yml
- name: Setup Node
  uses: actions/setup-node@v4
  with:
    node-version: '18'
    registry-url: 'https://npm.pkg.github.com'
    scope: '@bcentdev'

- name: Publish to GitHub Packages
  run: npm publish
  env:
    NODE_AUTH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

---

## 🔄 Cambiar entre registros

### De npmjs.org a GitHub Packages

```bash
cp package.github.json package.json
cp .npmrc.github .npmrc
git add package.json .npmrc
git commit -m "chore: switch to GitHub Packages"
```

### De GitHub Packages a npmjs.org

```bash
git checkout package.json  # Restaurar versión original
rm .npmrc                  # Eliminar config GitHub
git commit -m "chore: switch to npmjs.org"
```

---

## 📝 Mejores Prácticas

### Para npmjs.org
1. ✅ Usa nombres sin scope o scope tuyo (`@tu-username/ultra`)
2. ✅ Marca paquetes como públicos: `npm publish --access public`
3. ✅ Verifica nombre disponible: `npm search ultra-bundler`
4. ✅ README bien documentado (aparece en npmjs.com)

### Para GitHub Packages
1. ✅ Usa scope de tu org/usuario: `@bcentdev/ultra`
2. ✅ Configura visibilidad pública en repo
3. ✅ Documenta instalación (usuarios necesitan `.npmrc`)
4. ✅ Conecta package con repo en settings

---

## 🧪 Testing Local

Antes de publicar, prueba localmente:

```bash
# Instalar desde directorio local
npm install -g .

# Probar
ultra --version
ultra build --help

# Desinstalar
npm uninstall -g ultra-bundler  # o @bcentdev/ultra
```

---

## ❓ FAQ

**P: ¿Puedo cambiar después?**
R: Sí, pero el nombre del paquete cambiará. Los usuarios deberán actualizar.

**P: ¿GitHub Packages es gratis?**
R: Sí para paquetes públicos. Privados tienen límites según plan.

**P: ¿Los usuarios necesitan GitHub para instalar desde GitHub Packages?**
R: Solo para paquetes privados. Públicos solo necesitan `.npmrc`.

**P: ¿Cuál es más rápido?**
R: npmjs.org suele ser más rápido y tiene mejor CDN.

**P: ¿Puedo tener ambos nombres?**
R: Técnicamente sí, pero confunde a los usuarios. Elige uno.

---

## 🎯 Mi Recomendación

Para **Ultra**, te recomiendo **npmjs.org** porque:

1. ✅ Es un tool público que quieres que use mucha gente
2. ✅ Quieres aparecer en `npm search bundler`
3. ✅ Instalación más simple (sin configuración extra)
4. ✅ Más visibilidad en la comunidad
5. ✅ esbuild, swc, biome todos están en npmjs.org

Usa **GitHub Packages** solo si:
- Necesitas control total sobre distribución
- Planeas versiones privadas empresariales
- Tu proyecto es parte de un ecosistema GitHub existente

---

**Nota**: La configuración actual está lista para **npmjs.org**. Si quieres GitHub Packages, ejecuta:

```bash
cp package.github.json package.json
```
