use axum::{
    Json, Router, debug_handler,
    extract::rejection::JsonRejection,
    http::StatusCode,
    routing::{get, post},
};
use bs58;
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction};
use spl_token::instruction as token_instruction;

use serde_json::{Value, json};
use spl_token::instruction::{initialize_mint2, mint_to};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/keypair", post(generate_keypair))
        .route("/token/create", post(create_token))
        .route("/token/mint", post(token_mint))
        .route("/message/sign", post(message_sign))
        .route("/message/verify", post(message_verify))
        .route("/send/sol", post(transfer_sol))
        .route("/send/token", post(transfer_token));

    let port = std::env::var("PORT").unwrap_or("3000".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[debug_handler]
async fn generate_keypair() -> (StatusCode, Json<Value>) {
    let keypair = Keypair::new();

    if keypair.pubkey().to_string().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Failed to generate keypair" })),
        );
    }

    let data = json!({
        "success": true,
        "data": {
            "pubkey": keypair.pubkey().to_string(),
            "secret": bs58::encode(keypair.to_bytes()).into_string()
        }
    });

    (StatusCode::OK, Json(data))
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenDetails {
    mintAuthority: String,
    mint: String,
    decimals: u8,
}

#[debug_handler]
async fn create_token(
    payload: Result<Json<TokenDetails>, JsonRejection>,
) -> (StatusCode, Json<Value>) {
    let token_details = match payload {
        Ok(Json(details)) => details,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid request body" })),
            );
        }
    };

    if token_details.mintAuthority.is_empty()
        || token_details.mint.is_empty()
        || token_details.decimals == 0
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Missing required fields" })),
        );
    }
    let mint = match token_details.mint.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid sender address" })),
            );
        }
    };
    let mint_authority = match token_details.mintAuthority.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid sender address" })),
            );
        }
    };

    let ix = initialize_mint2(
        &spl_token::ID,
        &mint,
        &mint_authority,
        Some(&mint_authority),
        token_details.decimals,
    );
    match ix {
        Ok(instr) => {
            let accounts: Vec<Value> = instr
                .accounts
                .into_iter()
                .map(|meta| {
                    json!({
                        "pubkey": meta.pubkey.to_string(),
                        "is_signer": meta.is_signer,
                        "is_writable": meta.is_writable
                    })
                })
                .collect();
            let ix_data = instr.data;
            return (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": {
                        "program_id": instr.program_id.to_string(),
                        "accounts": accounts,
                        "instruction_data": ix_data
                    }
                })),
            );
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "Hello",
            })),
        ),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenMint {
    mint: String,
    destination: String,
    authority: String,
    amount: u64,
}

#[debug_handler]
async fn token_mint(payload: Result<Json<TokenMint>, JsonRejection>) -> (StatusCode, Json<Value>) {
    let mint_details = match payload {
        Ok(Json(details)) => details,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid request body" })),
            );
        }
    };

    if mint_details.mint.is_empty()
        || mint_details.destination.is_empty()
        || mint_details.authority.is_empty()
        || mint_details.amount == 0
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Missing required fields" })),
        );
    }
    let mint_key = match mint_details.mint.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid sender address" })),
            );
        }
    };
    let authority_pubkey = match mint_details.authority.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid sender address" })),
            );
        }
    };

    let destination_pubkey = match mint_details.destination.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid sender address" })),
            );
        }
    };

    let ix = mint_to(
        &spl_token::ID,
        &mint_key,
        &destination_pubkey,
        &authority_pubkey,
        &[&authority_pubkey],
        mint_details.amount,
    );
    match ix {
        Ok(instr) => {
            let accounts: Vec<Value> = instr
                .accounts
                .into_iter()
                .map(|meta| {
                    json!({
                        "pubkey": meta.pubkey.to_string(),
                        "is_signer": meta.is_signer,
                        "is_writable": meta.is_writable
                    })
                })
                .collect();

            let instruction_data = instr.data;

            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": {
                        "program_id": instr.program_id.to_string(),
                        "accounts": accounts,
                        "instruction_data": instruction_data
                    }
                })),
            )
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "Hello",
            })),
        ),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MessageSign {
    message: String,
    secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MessageVerify {
    message: String,
    signature: String,
    pubkey: String,
}

#[debug_handler]
async fn message_verify(
    payload: Result<Json<MessageVerify>, JsonRejection>,
) -> (StatusCode, Json<Value>) {
    let verify_details = match payload {
        Ok(Json(details)) => details,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid request body" })),
            );
        }
    };

    if verify_details.message.is_empty()
        || verify_details.signature.is_empty()
        || verify_details.pubkey.is_empty()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Missing required fields" })),
        );
    }
    let pubkey = match verify_details.pubkey.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid public key format"
                })),
            );
        }
    };

    let signature = match verify_details
        .signature
        .parse::<solana_sdk::signature::Signature>()
    {
        Ok(sig) => sig,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid signature format"
                })),
            );
        }
    };

    let message_bytes = verify_details.message.as_bytes();
    let is_valid = signature.verify(&pubkey.to_bytes(), message_bytes);

    let response = json!({
        "success": true,
        "data": {
            "valid": is_valid,
            "message": verify_details.message,
            "pubkey": verify_details.pubkey
        }
    });

    (StatusCode::OK, Json(response))
}

#[debug_handler]
async fn message_sign(
    payload: Result<Json<MessageSign>, JsonRejection>,
) -> (StatusCode, Json<Value>) {
    let sign_details = match payload {
        Ok(Json(details)) => details,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid request body" })),
            );
        }
    };

    if sign_details.message.is_empty() || sign_details.secret.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Missing required fields" })),
        );
    }
    let secret_bytes = match bs58::decode(&sign_details.secret).into_vec() {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid secret key format" })),
            );
        }
    };

    let keypair = match Keypair::from_bytes(&secret_bytes) {
        Ok(kp) => kp,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid keypair bytes" })),
            );
        }
    };

    let signature = keypair.sign_message(sign_details.message.as_bytes());

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "signature": signature.to_string(),
                "public_key": keypair.pubkey().to_string(),
                "message": sign_details.message
            }
        })),
    )
}

#[debug_handler]
async fn transfer_sol(
    payload: Result<Json<TransferSol>, JsonRejection>,
) -> (StatusCode, Json<Value>) {
    let details = match payload {
        Ok(Json(details)) => details,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid request body" })),
            );
        }
    };

    if details.from.is_empty() || details.to.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Missing required fields" })),
        );
    }
    let from_pubkey = match details.from.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid sender address" })),
            );
        }
    };

    let to_pubkey = match details.to.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid recipient address" })),
            );
        }
    };

    if details.lamports == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Amount must be greater than 0" })),
        );
    }

    let instruction = system_instruction::transfer(&from_pubkey, &to_pubkey, details.lamports);

    let response = TransferSolResponse {
        success: true,
        data: TransferSolData {
            program_id: instruction.program_id.to_string(),
            accounts: instruction
                .accounts
                .iter()
                .map(|a| a.pubkey.to_string())
                .collect(),
            instruction_data: bs58::encode(instruction.data).into_string(),
        },
    };

    (StatusCode::OK, Json(json!(response)))
}

#[derive(Debug, Deserialize)]
struct Address {
    address: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TransferToken {
    owner: String,
    destination: String,
    mint: String,
    amount: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct TransferSol {
    from: String,
    to: String,
    lamports: u64,
}

#[derive(Debug, Serialize)]
struct TransferSolResponse {
    success: bool,
    data: TransferSolData,
}

#[derive(Debug, Serialize)]
struct TransferSolData {
    program_id: String,
    accounts: Vec<String>,
    instruction_data: String,
}

#[debug_handler]
async fn transfer_token(
    payload: Result<Json<TransferToken>, JsonRejection>,
) -> (StatusCode, Json<Value>) {
    let details = match payload {
        Ok(Json(details)) => details,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid request body" })),
            );
        }
    };

    if details.owner.is_empty() || details.destination.is_empty() || details.mint.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Missing required fields" })),
        );
    }
    let from_pubkey = match details.owner.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid sender address" })),
            );
        }
    };

    let to_pubkey = match details.destination.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid recipient address" })),
            );
        }
    };

    let mint_pubkey = match details.mint.parse::<Pubkey>() {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": "Invalid mint address" })),
            );
        }
    };

    // if details.amount == 0 {
    //     return (
    //         StatusCode::BAD_REQUEST,
    //         Json(json!({ "success": false, "error": "Amount must be greater than 0" })),
    //     );
    // }

    let instruction = token_instruction::transfer(
        &spl_token::id(),
        &from_pubkey,
        &to_pubkey,
        &from_pubkey,
        &[],
        details.amount,
    );
    match instruction {
        Ok(ix) => {
            let response = TransferTokenResponse {
                success: true,
                data: TransferTokenData {
                    program_id: ix.program_id.to_string(),
                    accounts: ix
                        .accounts
                        .iter()
                        .map(|a| AccountMeta {
                            pubkey: a.pubkey.to_string(),
                            is_signer: a.is_signer,
                        })
                        .collect(),
                    instruction_data: bs58::encode(ix.data).into_string(),
                },
            };

            (StatusCode::OK, Json(json!(response)))
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Amount must be greater than 0" })),
        ),
    }
}

#[derive(Debug, Serialize)]
struct TransferTokenResponse {
    success: bool,
    data: TransferTokenData,
}

#[derive(Debug, Serialize)]
struct AccountMeta {
    pubkey: String,
    is_signer: bool,
}

#[derive(Debug, Serialize)]
struct TransferTokenData {
    program_id: String,
    accounts: Vec<AccountMeta>,
    instruction_data: String,
}
