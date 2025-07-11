---
comments: false
hide:
  - navigation
  - toc
---

# Witaj na stronie poświęconej me3

**me<sup>3</sup>** to framework zaprojektowany do modyfikacji gier podczas ich działania, ze szczególnym uwzględnieniem ELDEN RING oraz innych tytułów studia FROMSOFTWARE. Jest następcą [ModEngine 2](https://github.com/soulsmods/ModEngine2).

[Pobierz :fontawesome-solid-download:](https://github.com/garyttierney/me3/releases/latest){ .md-button .md-button--primary }

## Instalacja

=== ":fontawesome-brands-windows: Windows"

    **Instalacja jednym kliknięciem:**

    Pobierz najnowszą wersję pliku me3_installer.exe ze strony [GitHub releases](https://github.com/garyttierney/me3/releases/latest) i postępuj zgodnie z kreatorem instalacji.

    **Instalacja ręczna:**

    1. Pobierz [wersję dla systemu Windows](https://github.com/garyttierney/me3/releases/latest).
    2. Wypakuj ją do wybranego lokalnego folderu (nie synchronizowanego z OneDrive lub podobnym programem).

=== ":fontawesome-brands-linux: Linux / Steam Deck"

    **Instalacja jednym poleceniem:**
    ```bash
    curl --proto '=https' --tlsv1.2 -sSfL https://github.com/garyttierney/me3/releases/latest/download/installer.sh | sh
    ```

    **Instalacja ręczna:**

    1. Pobierz [wersję dla systemu Linux](https://github.com/garyttierney/me3/releases/latest).
    2. Wypakuj ją do wybranego lokalnego folderu:
       ```bash
       tar -xzf me3-linux-amd64.tar.gz
       cd me3-linux-amd64
       ./bin/me3 --windows-binaries-dir ./bin/win64 info
       ```

=== ":fontawesome-brands-apple: macOS"

    me3 działa na macOS przez [CrossOver®](https://www.codeweavers.com/crossover). Postępuj zgodnie z instrukcjami instalacji dla Windows w swoim środowisku CrossOver.

## Podręcznik szybkiej instalacji

### 1. Instalacja

Wybierz swój system operacyjny z zakładek powyżej i postępuj zgodnie z instrukcjami instalacji.

### 2. Konfigurowanie profili modów

- [Tworzenie profili modów](user-guide/creating-mod-profiles.md) - Dowiedz się, jak pobierać i konfigurować mody.
- [Dokumentacja konfiguracji](configuration-reference.md) - Pełna lista opcji konfiguracji.

### 3. Uruchom profil moda

Uruchom skonfigurowany profil `.me3` lub domyślny profil z menu Start (Windows) lub w wierszu poleceń:

```shell
me3 launch --auto-detect -p eldenring-default
```

## Potrzebujesz pomocy?

- **Jesteś początkującym użytkownikiem?** Zacznij od naszego [Przewodnika użytkownika](user-guide/installation.md)
- **Napotkałeś problem?** Odwiedź sekcję [Rozwiązywanie problemów](user-guide/troubleshooting.md)
- **Znalazłeś błąd?** [Zgłoś go](https://github.com/garyttierney/me3/discussions/categories/bug-reports)
- **Masz pomysł na nową funkcję** [Zaproponuj ją](https://github.com/garyttierney/me3/discussions/categories/ideas)
