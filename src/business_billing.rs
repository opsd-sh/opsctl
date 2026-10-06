//! Billing commands for standalone business administrators.
use crate::{
    auth::ServerUrl,
    authenticated_client,
    billing::{InvoicePage, LinePage, PaymentMethodPage, save_pdf},
    print_json,
    website::WebsiteUrl,
};
use clap::Subcommand;
use opsd::types::{BusinessId, InvoiceId, SetDefaultPaymentMethodRequest, StripePaymentMethodId};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(crate) enum BusinessBillingCommand {
    /// Check the payer's default payment method live in Stripe.
    Status { business_id: BusinessId },
    /// Open the website to set up billing details.
    Setup { business_id: BusinessId },
    /// List a page of live saved payment methods.
    PaymentMethods {
        business_id: BusinessId,
        #[command(flatten)]
        page: PaymentMethodPage,
    },
    /// Select a saved payment method. Does not charge, retry or lift suspension.
    SetDefaultPaymentMethod {
        business_id: BusinessId,
        payment_method_id: StripePaymentMethodId,
    },
    /// List a page of saved invoices, newest first.
    Invoices {
        business_id: BusinessId,
        #[command(flatten)]
        page: InvoicePage,
    },
    /// List a page of detailed charges (GBP pence, before discounts and tax).
    InvoiceLines {
        business_id: BusinessId,
        invoice_id: InvoiceId,
        #[command(flatten)]
        page: LinePage,
    },
    /// Download a PDF to a new file; existing files are never overwritten.
    InvoicePdf {
        business_id: BusinessId,
        invoice_id: InvoiceId,
        #[arg(long)]
        output: PathBuf,
    },
}

pub(crate) async fn execute(
    command: BusinessBillingCommand,
    server_url: &ServerUrl,
    website_url: &WebsiteUrl,
) -> Result<(), Box<dyn std::error::Error>> {
    if let BusinessBillingCommand::Setup { business_id } = command {
        crate::website::open_billing_setup(website_url, &business_id.to_string());
        return Ok(());
    }
    let client = authenticated_client(server_url)?;
    match command {
        BusinessBillingCommand::Status { business_id } => {
            print_json(&client.get_billing_status(business_id).await?)?
        }
        BusinessBillingCommand::Setup { .. } => {
            unreachable!("setup handled without API authentication")
        }
        BusinessBillingCommand::PaymentMethods { business_id, page } => print_json(
            &client
                .list_payment_methods(business_id, &page.into())
                .await?,
        )?,
        BusinessBillingCommand::SetDefaultPaymentMethod {
            business_id,
            payment_method_id,
        } => {
            client
                .set_default_payment_method(
                    business_id,
                    &SetDefaultPaymentMethodRequest { payment_method_id },
                )
                .await?
        }
        BusinessBillingCommand::Invoices { business_id, page } => {
            print_json(&client.list_invoices(business_id, &page.into()).await?)?
        }
        BusinessBillingCommand::InvoiceLines {
            business_id,
            invoice_id,
            page,
        } => print_json(
            &client
                .list_invoice_lines(business_id, invoice_id, &page.into())
                .await?,
        )?,
        BusinessBillingCommand::InvoicePdf {
            business_id,
            invoice_id,
            output,
        } => {
            let bytes = client.download_invoice_pdf(business_id, invoice_id).await?;
            save_pdf(&output, &bytes)?;
        }
    }
    Ok(())
}
