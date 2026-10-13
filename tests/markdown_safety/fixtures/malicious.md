# Safe heading

<script>window.__note_xss = true</script>
<img src="https://example.com/safe.png" onerror="window.__note_xss = true" style="position:fixed" srcset="file:///secret 2x">
<a href="jav&#x61;script:window.__note_xss=true" onclick="window.__note_xss=true">entity script</a>
<a href="java&#9;script:window.__note_xss=true">tab script</a>
<a href="java&#10;script:window.__note_xss=true">newline script</a>
<a href="data:text/html,<script>alert(1)</script>">data link</a>
<a href="file:///C:/secret">file link</a>
<a href="tauri://localhost">Tauri link</a>
<a href="asset://localhost/secret">asset link</a>
<a href="mercury://settings">custom link</a>
<a href="/settings">relative link</a>
<a href="//example.com">protocol relative link</a>
<img src="data:image/svg+xml,<svg onload='alert(1)'></svg>">
<img src="data:image/png;base64,iVBORw0KGgo=">
<img src="file:///C:/secret.png">
<iframe srcdoc="<script>alert(1)</script>"></iframe>
<object data="https://example.com/evil"></object>
<embed src="https://example.com/evil">
<style>body { display:none }</style>
<link rel="stylesheet" href="https://example.com/evil.css">
<meta http-equiv="refresh" content="0;url=https://example.com">
<base href="https://example.com">
<svg><a xlink:href="javascript:alert(1)"><text>SVG</text></a></svg>
<math><mtext><img src=x onerror="window.__note_xss=true"></mtext></math>
<form id="app" name="__TAURI_INTERNALS__" action="https://example.com">
  <input type="text" name="secret" autofocus onfocus="alert(1)">
  <input type="submit" formaction="https://example.com">
  <button>send</button><textarea>secret</textarea><select><option>secret</option></select>
</form>
<input type="checkbox" checked name="unsafe" onclick="alert(1)">
<p id="__TAURI_INTERNALS__" name="location" data-danger="1" aria-hidden="true" style="display:none" onmouseover="alert(1)">safe paragraph</p>
<code class="hidden fixed language-js" style="display:none">safe code</code>

[Markdown script](javascript:alert%281%29)
[Markdown data](data:text/html,evil)
![Markdown file](file:///C:/secret.png)

<math><mtext><table><mglyph><style><!--</style><img title="--><img src=x onerror=window.__note_xss=true>">

<svg><style><a id="</style><img src=x onerror=window.__note_xss=true>"></a></style></svg>
