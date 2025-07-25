# Znane problemy i FAQ

Ta sekcja zawiera listę typowych problemów i często zadawanych pytań, opartych na opiniach użytkowników i zgłoszeniach z GitHub. Instrukcje dotyczące rozwiązywania problemów znajdują się w sekcji [Rozwiązywanie problemów](troubleshooting.md).

## FAQ

### Gdzie znajdują się foldery z profilami modów?

Domyślne foldery profili modów są zarządzane w `$HOME/.config/me3/profiles` w systemie Linux oraz `%LOCALAPPDATA%\garyttierney\me3\config\profiles` w systemie Windows.

### Launcher nie uruchamia się. Co powinienem zrobić?

Dokładnie sprawdź plik konfiguracyjny, ustawienia antywirusa i zapoznaj się z sekcją [Rozwiązywanie problemów](troubleshooting.md).

### Jak zainstalować mody?

Zapoznaj się z dokumentacją na temat [tworzenia profili modów](./creating-mod-profiles.md)

### Gdzie znajdę mój plik konfiguracyjny?

Globalny plik konfiguracyjny dla me3 znajduje się w `$HOME/.config/me3/me3.toml` w systemie Linux oraz `%LOCALAPPDATA%\garyttierney\me3\config\me3.toml` w systemie Windows.

### Jak używać niestandardowej ścieżki do gry z me3?

Możesz użyć komendy `me3 launch` do wskazania niestandardowego pliku wykonywalnego gry. Na przykład:

```shell
> $ me3 launch --auto-detect --skip-steam-init --exe-path="C:/game-archive/eldenring.exe"
```

## Znane problemy

### (Steam Deck) Gra nie uruchamia się, gdy jest zainstalowana na karcie SD

!!! bug "me3 nie znajduje prefiksu zgodności dla gier zainstalowanych na karcie SD."
!!! success "Przenieś instalację gry do pamięci głównej lub utwórz dowiązanie symboliczne do folderu compat w bibliotece Steam."

### me3 jest poddawane kwarantannie przez oprogramowanie antywirusowe

!!! bug "Niektóre programy antywirusowe mogą oznaczać launcher lub hosta modów jako złośliwe oprogramowanie."
!!! success "Dodaj wyjątek dla launchera/hosta modów w swoim antywirusie. Pobieraj tylko z oficjalnych źródeł."

### Gra nadal jest uruchomiona w Steam po wyjściu z menu

!!! bug "Procesy gry lub launchera mogą nie zawsze zamykać się prawidłowo."
!!! success "Ręcznie zakończ pozostałe procesy gry (np. za pomocą Menedżera zadań w systemie Windows)."

---

Aby uzyskać więcej pomocy, odwiedź sekcję [Rozwiązywanie problemów](troubleshooting.md) lub dołącz do dyskusji społeczności.
