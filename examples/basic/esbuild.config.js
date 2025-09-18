import { build } from 'esbuild'
import { copyFileSync, mkdirSync, writeFileSync } from 'fs'
import { readFileSync } from 'fs'

const buildEsbuild = async () => {
  const startTime = performance.now()

  try {
    // Create output directory
    mkdirSync('./dist-esbuild', { recursive: true })

    // Build JavaScript
    const jsResult = await build({
      entryPoints: ['./main.js'],
      bundle: true,
      minify: true,
      format: 'esm',
      target: 'es2020',
      outdir: './dist-esbuild',
      write: true,
      metafile: true
    })

    // Build CSS separately (esbuild can't auto-discover CSS from JS imports without explicit imports)
    const cssFiles = ['./styles.css', './styles.module.css', './advanced-styles.css']
    let bundledCSS = '/* esbuild CSS bundle */\n'

    for (const cssFile of cssFiles) {
      try {
        const cssResult = await build({
          entryPoints: [cssFile],
          bundle: true,
          minify: true,
          write: false,
          loader: { '.css': 'css' }
        })

        if (cssResult.outputFiles && cssResult.outputFiles[0]) {
          bundledCSS += `/* From: ${cssFile} */\n`
          bundledCSS += cssResult.outputFiles[0].text + '\n'
        }
      } catch (error) {
        console.warn(`Warning: Could not process ${cssFile}:`, error.message)
      }
    }

    // Write bundled CSS
    writeFileSync('./dist-esbuild/bundle.css', bundledCSS)

    // Copy and update HTML
    let htmlContent = readFileSync('./index.html', 'utf8')
    htmlContent = htmlContent
      .replace('./main.js', './main.js')
      .replace('./styles.css', './bundle.css')

    writeFileSync('./dist-esbuild/index.html', htmlContent)

    const buildTime = performance.now() - startTime

    console.log(`📊 esbuild Statistics:`)
    console.log(`  • Build time: ${buildTime.toFixed(2)}ms`)
    console.log(`  • JS modules: ${Object.keys(jsResult.metafile.inputs).length}`)
    console.log(`  • CSS files: ${cssFiles.length}`)

    return { buildTime, success: true }

  } catch (error) {
    console.error('esbuild failed:', error)
    return { buildTime: performance.now() - startTime, success: false }
  }
}

buildEsbuild()