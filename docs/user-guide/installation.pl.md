# Instalacja

Ten przewodnik zawiera instrukcje krok po kroku dotyczące instalacji `me3` -  programu ładującego mody do gier FROMSOFTWARE. Po ukończeniu tego przewodnika `me3` będzie działać na Twoim systemie i będziesz mógł korzystać z profili modów w ELDEN RING.

## Uruchamianie instalatora

=== ":fontawesome-brands-windows: Windows"

    Najprostszym sposobem instalacji me3 w systemie Windows jest skorzystanie z instalatora dołączonego do każdej wersji. Ta metoda zapewnia prawidłowe umieszczenie i skonfigurowanie wszystkich niezbędnych plików na Twoim systemie.

    <h3>1. Pobierz instalator</h3>

    Zacznij od pobrania instalatora z oficjalnego źródła. Przejdź do strony [GitHub releases](https://github.com/garyttierney/me3/releases/latest), która zawiera listę wszystkich dostępnych wersji.

    Po wybraniu wersji poszukaj pliku `me3_installer.exe` w sekcji "Assets" i pobierz go.

    ??? warning "Ostrzeżenia zabezpieczeń przeglądarki (Kliknij, aby otworzyć)"

        Twoja przeglądarka internetowa może wyświetlić ostrzeżenie podczas pobierania plików wykonywalnych (`.exe`), sugerując, że plik może być szkodliwy. Pliki pobierane bezpośrednio z oficjalnego repozytorium `me3` na GitHubie są zasadniczo bezpieczne. Wybierz opcję "Zachowaj" lub "Pobierz mimo to" (dokładne sformułowanie różni się w zależności od przeglądarki). Za każdym razem upewnij się, czy źródłem pobierania jest `https://github.com/garyttierney/me3/`.

    <h3>2. Uruchom instalator</h3>

    Po zakończeniu pobierania pliku `me3_installer.exe` znajdź go w folderze Pobrane (lub w miejscu, w którym go zapisałeś) i kliknij go dwukrotnie, aby uruchomić kreatora instalacji.

    Kreator instalacji poprowadzi Cię przez proces konfiguracji. Po wybraniu miejsca instalacji kliknij **Instaluj**, aby rozpocząć kopiowanie plików. Pasek postępu pokaże status instalacji, a po jej zakończeniu pojawi się ekran końcowy. Kliknij "Zakończ", aby zamknąć instalator.

=== ":fontawesome-brands-linux: Linux"

    me3 zawiera instalator w postaci skryptu powłoki dla systemu Linux, który pobiera przenośną wersję programu z GitHuba, rozpakowuje pliki do odpowiednich lokalizacji i może być uruchomiony jako tradycyjny instalator za pomocą jednej linii polecenia:

    <h3>1. Uruchom skrypt instalatora</h3>

    ```bash
    curl --proto '=https' --tlsv1.2 -sSfL https://github.com/garyttierney/me3/releases/latest/download/installer.sh | sh
    ```

    <h3>2. Dodaj plik wykonywalny me3 do zmiennej PATH</h3>

    Upewnij się, że `me3` jest dostępny w Twojej zmiennej PATH, sprawdzając, czy polecenie `me3 info` działa poprawnie. Jeśli nie, zaktualizuj zmienną środowiskową `PATH`, dodając do niej katalog `$HOME/.local/bin`.

## Weryfikacja instalacji

me3 domyślnie utworzy zestaw pustych profili dla ELDEN RING.
Sprawdź poprawność instalacji, uruchamiając za pomocą me3 pusty profil – możesz to zrobić z poziomu wiersza poleceń lub przez dwukrotne kliknięcie pliku .me3 w powłoce systemu Windows.:

```shell
> $ me3 launch --auto-detect -p eldenring-default
```

Zobacz `me3 launch --help`, aby uzyskać informacje na temat parametrów `auto-detect` i innych.

## Co dalej?

Zapoznaj się z [Dokumentacją konfiguracji](../configuration-reference.md) i sekcją [Tworzenie profili modów](./creating-mod-profiles.md), aby dowiedzieć się jak zacząć używać modów z me3.
