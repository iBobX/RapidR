' tests/web_bundle_csp.mjs (docs/security-audit.md SEC-12 / SEC-15): an
' RWEBVIEW's Html runs its own scripts — in its sandboxed frame, at an opaque
' origin, under the frame's rules rather than the page's policy — and can't
' reach the program's page.
$APPTYPE WEB

CREATE Form AS QFORM
  Caption = "webview"
  Width = 420
  Height = 260
  CREATE Lbl AS QLABEL
    Left = 8
    Top = 8
    Width = 300
    Caption = "-"
  END CREATE
  CREATE Web AS RWEBVIEW
    Left = 8
    Top = 30
    Width = 380
    Height = 180
  END CREATE
END CREATE

Web.SetHtml("<p id='p'>before</p><script>document.getElementById('p').textContent = 'script ran'; try { parent.document.title = 'pwned'; document.body.dataset.reach = 'parent' } catch (e) { document.body.dataset.reach = 'blocked' } try { localStorage.setItem('k', '1'); document.body.dataset.storage = 'yes' } catch (e) { document.body.dataset.storage = 'no' }</script>")
Lbl.Caption = "set"
Form.ShowModal
