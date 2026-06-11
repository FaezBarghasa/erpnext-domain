use rust_decimal::Decimal;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::types::RecordId;
use surrealdb::Surreal;
use erp_accounting::posting::{GLEntry, LedgerPostingEngine};
use erp_accounting::balance_sheet::BalanceSheetReport;

#[tokio::test]
async fn test_ledger_posting_integration() {
    // Attempt to connect to a running SurrealDB instance, skip test if not running.
    let db_res = Surreal::new::<Ws>("127.0.0.1:8000").await;
    let db = match db_res {
        Ok(db) => db,
        Err(_) => {
            println!("SurrealDB not running at 127.0.0.1:8000, skipping integration test.");
            return;
        }
    };

    // Signin
    if let Err(e) = db.signin(surrealdb::opt::auth::Root {
        username: "root".to_string(),
        password: "root".to_string(),
    }).await {
        println!("Failed to signin: {}, skipping database test.", e);
        return;
    }

    let ns = "test_ledger_ns";
    let database = "test_ledger_db";

    // Set namespace and database
    if let Err(e) = db.use_ns(ns).use_db(database).await {
        println!("Failed to use namespace/db: {}, skipping database test.", e);
        return;
    }

    // Clean tables and prepare accounts
    let clean_query = "REMOVE TABLE gl_entry; REMOVE TABLE account; REMOVE TABLE stock_transfer; REMOVE TABLE sales_invoice;";
    let _ = db.query(clean_query).await;

    // Create accounts
    let create_accounts = "
        CREATE account:asset SET name = 'Asset Account', balance = 0.0;
        CREATE account:revenue SET name = 'Revenue Account', balance = 0.0;
    ";
    let res = db.query(create_accounts).await.expect("Failed to create accounts");
    res.check().expect("Failed accounts check");

    // Prepare GLEntries
    let entries = vec![
        GLEntry {
            account: RecordId::parse_simple("account:asset").unwrap(),
            debit: Decimal::new(150, 0), // 150.00
            credit: Decimal::ZERO,
            voucher_type: "sales_invoice".to_string(),
            voucher_no: "SINV_100".to_string(),
            cost_center: None,
        },
        GLEntry {
            account: RecordId::parse_simple("account:revenue").unwrap(),
            debit: Decimal::ZERO,
            credit: Decimal::new(150, 0), // 150.00
            voucher_type: "sales_invoice".to_string(),
            voucher_no: "SINV_100".to_string(),
            cost_center: None,
        },
    ];

    // Create the voucher first (needed for the relate graph edge)
    let create_voucher = "CREATE sales_invoice:SINV_100 SET total = 150.0;";
    let _ = db.query(create_voucher).await.expect("Failed to create voucher");

    // Commit Transaction using LedgerPostingEngine
    LedgerPostingEngine::commit_transaction(
        &db,
        ns,
        database,
        "sales_invoice",
        "SINV_100",
        &entries,
    ).await.expect("Failed to commit ledger entries");

    // Verify account balances
    let verify_query = "SELECT id, balance FROM account;";
    let mut response = db.query(verify_query).await.expect("Failed verify query");
    #[derive(serde::Deserialize, Debug)]
    struct AccountBalance {
        balance: Decimal,
    }
    let raw_balances: Vec<serde_json::Value> = response.take(0).expect("Failed take balance");
    let balances: Vec<AccountBalance> = serde_json::from_value(serde_json::Value::Array(raw_balances))
        .expect("Failed to deserialize balances");
    assert_eq!(balances.len(), 2);
    // Since we don't know the exact order returned:
    let mut sum_balances = Decimal::ZERO;
    for acc in balances {
        sum_balances += acc.balance.abs();
    }
    assert_eq!(sum_balances, Decimal::new(300, 0)); // 150 debit + 150 credit (abs sum)

    // Verify balance sheet report rollup
    let report = BalanceSheetReport::fetch_and_compute(&db, ns, database, "2026-01-01", "2026-12-31")
        .await
        .expect("Failed to fetch and compute balance sheet");

    assert_eq!(report.assets, Decimal::new(150, 0));
    assert_eq!(report.liabilities, Decimal::ZERO);
}
