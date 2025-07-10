# Rozwiązywanie problemów

Problemy podczas początkowej konfiguracji modów zdarzają się często. Ta sekcja przeprowadzi Cię przez diagnozowanie i rozwiązywanie najczęstszych problemów, które mogą wystąpić podczas korzystania z `me3`.

!!! warning "Pierwszy krok: typowe problemy"
    Zanim zagłębisz się w szczegóły, warto szybko zweryfikować kilka typowych źródeł błędów. Często problemem jest prosta **literówka** w pliku `.me3`, w `id` lub w słowie kluczowym, takim jak `packages` czy `path`. Inną częstą pułapką są **niepoprawne ścieżki**. Pamiętaj, że wszystkie ścieżki `path` dla `[[packages]]` i `[[natives]]` są **względne** w stosunku do lokalizacji pliku `.me3`, więc upewnij się, że poprawnie wskazują pliki modyfikacji.

---

## Zasoby

- Listę bieżących błędów i często zadawanych pytań znajdziesz w sekcji [Znane problemy i FAQ](./faq.md#known-issues).

## Częste problemy

### Ostrzeżenia programu antywirusowego

Pliki wykonywalne me3 są teraz podpisywane cyfrowo certyfikatem Certum, aby ograniczyć liczbę fałszywych alarmów. Jeśli Twój antywirus oznacza me3:

- Sprawdź, czy pobrane pliki pochodzą ze strony [GitHub releases](https://github.com/garyttierney/me3/releases).
- Dodaj instalator me3 i katalog instalacyjny me3 do wykluczeń antywirusa.

### Gra nie uruchamia się

- Upewnij się, że Steam jest uruchomiony przed uruchomieniem me3.
- Dokładnie sprawdź ścieżki wymienione w pliku .me3.
- (Windows) Uruchom (++windows+r++) `me3 info`, aby sprawdzić, czy instalacja przebiegła pomyślnie.
- (Linux) zweryfikuj, czy `windows_binaries_dir` jest ustawiony w pliku konfiguracyjnym (`~/.config/me3`).

## Nadal występują problemy?

Zgłoś błąd lub poproś o pomoc na [Forum dyskusyjnym](https://github.com/garyttierney/me3/discussions/).
