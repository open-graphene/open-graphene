# Swaplock crypto_api — test live po wdrożeniu, 29 września 2026

**7/7 metod crypto_api działa na node01 i node02**, w Node.js i Chromium na origin portalu. Wszystkie wywołania przechodzą przez produkcyjny transport i generowane kodeki SDK TypeScript.

Ponowiono pełny zestaw metod: **46/46 niebędących broadcastem w każdym z 4 przebiegów**, czyli 184 pozytywne wyniki, w tym 28 dla crypto. Dodatkowo **32/32 scenariusze**. Brak odmów dostępu i nieoczekiwanych błędów.

Przebiegi rozpoczęły się 2026-09-29T14:18:42.352Z, zakończyły 2026-09-29T14:18:47.575Z.

| Metoda | Node.js/Chromium × node01/node02 |
|---|---|
| `crypto.blind` | 4/4 PASS |
| `crypto.blind_sum` | 4/4 PASS |
| `crypto.verify_sum` | 4/4 PASS |
| `crypto.verify_range` | 4/4 PASS |
| `crypto.range_proof_sign` | 4/4 PASS |
| `crypto.verify_range_proof_rewind` | 4/4 PASS |
| `crypto.range_get_info` | 4/4 PASS |

## Zweryfikowane zachowanie

- Commitment dla wartości 7 i publicznego blind=1 ma 33 bajty; identyczny na obu węzłach i w obu runtime.
- blind_sum odejmuje ten sam blind i zwraca zero.
- verify_sum akceptuje zbilansowane commitments, odrzuca excess=1.
- range_proof_sign tworzy dowód, verify_range go akceptuje, a range_get_info zwraca zakres obejmujący wartość.
- verify_range_proof_rewind odzyskuje dokładnie wartość 7 i pierwotny blind.
- Uszkodzony dowód jest odrzucany.
- Błędny nonce jest odrzucany błędem RPC z natywnego `FC_ASSERT(secp256k1_rangeproof_rewind(...))`. Pierwotne oczekiwanie `success=false` w nowym teście negatywnym skorygowano po odczytaniu implementacji C++: `libraries/fc/src/crypto/elliptic_secp256k1.cpp:275`. Test rozpoznaje wyłącznie ten konkretny błąd; nie traktuje dowolnego wyjątku ani timeoutu jako sukcesu.
- Wartość **9007199254740993** (2^53+1) przechodzi commitment, proof, verify i rewind bez utraty precyzji.
- Dowód utworzony na jednym węźle jest poprawnie weryfikowany na drugim, w obu kierunkach.

Pozostałe scenariusze sprawdzają limit opłaty transferu, brak odbiorcy oraz null dla brakującego pokoju/karty/bloku.

## Łączne pokrycie aktualnego SDK

Po połączeniu tego przebiegu z wcześniejszym, potwierdzonym broadcastem mamy funkcjonalne potwierdzenie **47/47 metod RPC obecnego SDK**. Wcześniejszy transfer: `93d5cb63619b1e36d34c87e6a93dd30cc7d6d62a`, [blok 1202081](https://portal.swaplock.chainpool.online/block/1202081). **Broadcastu nie powtarzano po wdrożeniu crypto_api.**

Ten test nie wysyła transakcji, nie używa kluczy genesis i nie zmienia konfiguracji infrastruktury. Blind, nonce i wartości w raporcie są publicznymi danymi syntetycznymi. Dane pokojów/grantów pochodzą z wcześniejszych fixture; zlecenie rynkowe zostało już anulowane, więc odczyty otwartych zleceń mogą teraz być puste. Wcześniejszy raport dokumentuje ich dodatnie, niepuste wyniki.

Pełne pokrycie metod nie oznacza pełnego pokrycia wszystkich wejść ani obsługi wszystkich operacji protokołu przez serializer FC TypeScript.

[Raport JSON](TYPESCRIPT-SWAPLOCK-CRYPTO-LIVE-2026-09-29.json) · [Wcześniejszy raport i broadcast](TYPESCRIPT-SDK-LIVE-COVERAGE-2026-09-29.md)

## Powtórzenie

Z katalogu `open-graphene-packages/typescript`, po `pnpm build`:

```sh
node scripts/test-all-live.mjs /tmp/swaplock-node.json /tmp/swaplock-sdk-fixtures.json
node scripts/test-all-live.mjs /tmp/swaplock-browser.json /tmp/swaplock-sdk-fixtures.json --browser
```

Plik fixture zawiera publiczne ID: assetId=`1.3.100`, subjectRoomId=`1.23.267`, grantCardId=`1.26.126`.
