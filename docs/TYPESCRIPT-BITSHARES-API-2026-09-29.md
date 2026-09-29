# BitShares API TypeScript — 29 września 2026

Dodano pakiet `@open-graphene/chain-bitshares-api`, oparty na wspólnej specyfikacji i generowanych bindingach BitShares. Zakres odpowiada aktualnemu etapowi transferów TypeScript Swaplock, nie pełnemu SDK Rust.

## Dostępne API

- `BitSharesClient.connect`: transport WebSocket i sprawdzenie mainnetowego chain ID.
- `client.rpc.invoke`: 11 generowanych metod w database, history i network_broadcast.
- `prepareTransfer`: lookup kont, opłata z limitem, saldo, TaPoS i immutable snapshot.
- `PreparedTransfer.sign`: pojedynczy klucz spełniający active authority, digest BitShares, kontrola poprawności i kanoniczności podpisu.
- `BitSharesWifSigner`: lokalny WIF, kompaktowy podpis low-S oraz kanoniczne 32-bajtowe r/s wymagane przez legacy Graphene.
- `SignedTransfer.toJSON`, `broadcast`, `waitForInclusion`, `close`.
- Niejednoznaczny wynik broadcastu zwraca `BroadcastOutcomeUnknown`; brak automatycznego ponawiania.

Do specyfikacji dodano get_chain_id, get_accounts, get_required_fees i broadcast_transaction oraz mainnetowy chain ID z oryginalnego bitsharesjs-ws. Bindingi Rust i TypeScript zregenerowano z tego samego IR. Ekstrakcja używa przypiętej rewizji BitShares Core `b92b82ba3e57381d111f28383e8cfa89a8356966`.

## Weryfikacja

**44/44 sprawdzenia live**: 10 generowanych RPC odczytu/wyceny oraz porównanie FC z natywnym C++, na 2 publicznych endpointach × Node.js/Chromium:

- `wss://api.dex.trading/`
- `wss://public.xbts.io/ws`

Sprawdzono chain ID, globalny stan, konta i null slots, obiekty, salda, nagłówek i blok, zlecenia, historię i opłatę transferu. Serializację niepodpisanego transferu porównano bajt w bajt z `get_transaction_hex_without_sig`.

**Nie wysłano transakcji na mainnet BitShares.** Broadcast i wyszukiwanie włączenia do bloku przeszły przez lokalny harness RPC z rzeczywistym transportem, codecami, serializacją i podpisem. To nie jest dowód live broadcastu BitShares.

Testy lokalne: **24 Node, 442 Rust**, type fixtures, test FC/podpisów i import API w Chromium, Clippy, rustfmt oraz kontrola deterministycznej regeneracji. Testy obejmują niewłaściwy łańcuch, limit opłaty, brak konta, saldo, stary head, utratę odpowiedzi po wysłaniu i brak ponowienia oraz odrzucenie błędnego podpisu z zewnętrznego signera.

Reguła kanoniczności odpowiada wymaganiu `lenR === 32 && lenS === 32` w oryginalnym `bitsharesjs/lib/ecc/src/signature.js:79–84`. Podpisywanie używa przypiętej biblioteki noble; ponawianie nonce korzysta z deterministycznego extraEntropy. Nie deklarujemy identycznych bajtów podpisu z bitsharesjs. Dotychczasowy domyślny profil Swaplock low-S zachowuje swoje wektory.

## Ograniczenia

Natywne FC obejmuje transfer; inne operacje, memo encryption, multisig, reconnect i subskrypcje pozostają kolejnymi etapami. Ten klient sprawdza mainnetowy chain ID i prefiks BTS; inny testnet wymaga odpowiedniego profilu generowania. Pakiet jest częścią workspace, nie został opublikowany w npm.

[Wyniki JSON](TYPESCRIPT-BITSHARES-API-2026-09-29.json) · [Użycie pakietu](../open-graphene-packages/typescript/README.md#bitshares-api)
