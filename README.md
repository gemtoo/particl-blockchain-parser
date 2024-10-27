# particl-blockchain-parser
## RPC-based parser for [Particl Coin](https://github.com/particl/particl-core)
This application consists of 3 containers:
1. The `particl-blockchain-parser` itself. It connects to `particld` and collects information from it to `surrealdb`.
2. Particl Core daemon `particld`. A node on the Particl Coin network.
3. [SurrealDB](https://surrealdb.com). The database to store the collected information.

### Prerequisites:

1. Install Git + [Docker Compose](https://docs.docker.com/engine/install/)
2. [Install SurrealDB](https://surrealdb.com/install). [Learn SurrealQL](https://surrealdb.com/docs/surrealql).
3. Download and run the `particl-blockchain-parser` container set:
```
git clone --depth 1 https://github.com/gemtoo/particl-blockchain-parser.git
cd particl-blockchain-parser
docker compose up -d
docker compose logs -f
```
4. Now that all parts are working, wait for the full chain sync. Then connect to the database container:
```
surreal sql --conn http://localhost:8000
```
Run some SQL queries to get insights about the Particl blockchain.\
Use `example` namespace and `example` db. These are hardcoded values.
```
USE NS example DB example;
```
Select all blocks that were forged in coldstaking process:
```
SELECT * FROM blocks WHERE coldstaking != NONE;
```
Find Top-10 blocks by their transaction count:
```
SELECT height, count(tx) FROM blocks ORDER BY count DESC LIMIT 10;
```
Select all block heights where transaction count is more than 20:
```
SELECT height FROM blocks WHERE count(tx) > 20;
```
Count all distinct coldstakeaddresses:
```
count(array::distinct(SELECT coldstaking FROM blocks WHERE coldstaking != NONE));
```
Count all distinct hotstaking addresses:
```
count(array::distinct(SELECT VALUE array::first(tx.vout[1].scriptPubKey.addresses[0]) FROM blocks WHERE coldstaking = NONE && tx.vout[1].scriptPubKey.addresses[0] != NONE));
```
Count all transactions:
```
math::sum(SELECT VALUE count(tx) FROM blocks);
```