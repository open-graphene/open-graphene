# Natywna transakcja TypeScript na testnecie

**Transfer wykonany i nieodwracalny**, 2026-09-29, 13:35:54 UTC (15:35:54 CEST).

- TX ID: `1a5f3f4448f72561e136b27f408857ebe85af6d4`.
- Blok: [1201598](https://portal.swaplock.chainpool.online/block/1201598), transakcja 0.
- Nadawca: `swaplock` (`1.2.100`).
- Odbiorca: `registrar` (`1.2.101`).
- Kwota: **0,01 BTS**, opłata: **2 BTS** — tokeny testnetu.
- Zmiana salda nadawcy: −2,01 BTS; odbiorcy: +0,01 BTS.
- Oba węzły Chainpool potwierdziły ten sam blok i indeks transakcji.
- Drugi węzeł osiągnął `last_irreversible_block_num = 1201598`.

Widok bloku w portalu zweryfikowano również w Chromium: pokazuje jedną
transakcję TRANSFER, `swaplock` → `registrar`, `0.01000 BTS`, fee `2.00000 BTS`.

[Raport JSON](TYPESCRIPT-LIVE-TRANSFER-2026-09-29.json) zawiera podpisaną
transakcję, FC hex, identyfikator, potwierdzenia obu węzłów i zmiany sald.
Nie zawiera kluczy prywatnych.

Całą ścieżkę wykonał natywny TypeScript: generowane z IR serializery FC,
bezstratny JSON, nagłówek TaPoS, wycena opłat, podpis secp256k1 low-S,
WebSocket broadcast i odczyt potwierdzenia. Klucz aktywny pochodził z lokalnego
genesis wskazanego przez użytkownika. Nie użyto Rust/WASM ani bitsharesjs do
tworzenia, podpisywania lub wysyłania tej transakcji.

Przed broadcastem serwer potwierdził zgodność signed FC hex z własną
serializacją C++ (`get_transaction_hex`) i uprawnienia podpisu
(`verify_authority`). TX ID policzono z SHA256 unsigned FC, pierwsze 20 bajtów,
zgodnie z `transaction::id()` w kodzie protokołu.

Dodano pakiety `graphene-transport` i `graphene-chain-swaplock-api`,
generator `fc.ts`, subpath `@open-graphene/fc/signing`, testy i uruchamialny
[przykład](../open-graphene-packages/typescript/examples/live-transfer.mjs).
Offline sprawdzono publiczne wektory podpisów/FC Rust, recovery, błędne podpisy,
niezmienność przygotowanej transakcji, odpowiedzi RPC poza kolejnością,
timeout bez ponawiania i zamknięcie połączenia. Chromium wykonuje także
generowane FC i publiczne testy podpisu/recovery bez polyfilli Node.

To pierwsza kompletna ścieżka transferu. Obsługa FC innych operacji, szyfrowanie
memo, multisig, legacy podpisy BitShares, reconnect i subskrypcje pozostają
kolejnymi etapami. Broadcast w tym SDK oznacza przyjęcie żądania; potwierdzenie
w bloku jest osobnym krokiem, a nie skutkiem samego sukcesu RPC.
