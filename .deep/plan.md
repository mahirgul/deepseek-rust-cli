# Bug Fix Plan

## Kritik (Hemen Fixlenmeli)
- [x] config.json'daki model ismi yanlış: "deepseek-v4-pro" -> "deepseek-chat"
- [ ] .env dosyası git'e dahil mi? Kontrol edilip çıkarılacak
- [ ] max_iterations=5 tanımlı ama autonomous döngüde kontrol yok (sonsuz döngü riski)
- [ ] auto_approve değişkeni tanımlı ama hiç kullanılmamış (dead code)

## Orta
- [ ] system_prompt değişkeni mut tanımlanmış ama değişmiyor (clippy warning)
- [ ] git status komutu her ortamda çalışmayabilir, hata yakalanmamış
- [ ] load_config fonksiyonu config.json yoksa crash olabilir
- [ ] Interrupted ve Eof aynı işlemi yapıyor, gereksiz ayrım

## Düşük
- [ ] .env.example dosyasında gerçek API key olmadığından emin olunmalı
- [ ] Cargo.toml'da gereksiz bağımlılıklar olabilir
