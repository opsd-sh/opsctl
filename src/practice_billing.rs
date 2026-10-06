//! Billing commands for practice administrators.
use crate::{
    auth::ServerUrl,
    authenticated_client,
    billing::{InvoicePage, LinePage, PaymentMethodPage, save_pdf},
    print_json,
};
use clap::Subcommand;
use opsd::types::{InvoiceId, PracticeId, SetDefaultPaymentMethodRequest, StripePaymentMethodId};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(crate) enum PracticeBillingCommand {
    /// Check the payer's default payment method live in Stripe.
    Status { practice_id: PracticeId },
    /// List a page of live saved payment methods.
    PaymentMethods {
        practice_id: PracticeId,
        #[command(flatten)]
        page: PaymentMethodPage,
    },
    /// Select a saved payment method. Does not charge, retry or lift suspension.
    SetDefaultPaymentMethod {
        practice_id: PracticeId,
        payment_method_id: StripePaymentMethodId,
    },
    /// List a page of saved invoices, newest first.
    Invoices {
        practice_id: PracticeId,
        #[command(flatten)]
        page: InvoicePage,
    },
    /// List a page of detailed charges (GBP pence, before discounts and tax).
    InvoiceLines {
        practice_id: PracticeId,
        invoice_id: InvoiceId,
        #[command(flatten)]
        page: LinePage,
    },
    /// Download a PDF to a new file; existing files are never overwritten.
    InvoicePdf {
        practice_id: PracticeId,
        invoice_id: InvoiceId,
        #[arg(long)]
        output: PathBuf,
    },
}

pub(crate) async fn execute(
    command: PracticeBillingCommand,
    server_url: &ServerUrl,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = authenticated_client(server_url)?;
    match command {
        PracticeBillingCommand::Status { practice_id } => {
            print_json(&client.get_practice_billing_status(practice_id).await?)?
        }
        PracticeBillingCommand::PaymentMethods { practice_id, page } => print_json(
            &client
                .list_practice_payment_methods(practice_id, &page.into())
                .await?,
        )?,
        PracticeBillingCommand::SetDefaultPaymentMethod {
            practice_id,
            payment_method_id,
        } => {
            client
                .set_practice_default_payment_method(
                    practice_id,
                    &SetDefaultPaymentMethodRequest { payment_method_id },
                )
                .await?
        }
        PracticeBillingCommand::Invoices { practice_id, page } => print_json(
            &client
                .list_practice_invoices(practice_id, &page.into())
                .await?,
        )?,
        PracticeBillingCommand::InvoiceLines {
            practice_id,
            invoice_id,
            page,
        } => print_json(
            &client
                .list_practice_invoice_lines(practice_id, invoice_id, &page.into())
                .await?,
        )?,
        PracticeBillingCommand::InvoicePdf {
            practice_id,
            invoice_id,
            output,
        } => {
            let bytes = client
                .download_practice_invoice_pdf(practice_id, invoice_id)
                .await?;
            save_pdf(&output, &bytes)?;
        }
    }
    Ok(())
}
