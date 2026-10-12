use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("Guard111111111111111111111111111111111111111");

#[program]
pub mod liquidity_guard {
    use super::*;

    pub fn initialize_guard(
        ctx: Context<InitializeGuard>,
        max_drawdown_bps: u16,
        cooldown_slots: u64,
        twap_window_seconds: u32,
    ) -> Result<()> {
        require!(max_drawdown_bps <= 5000, GuardError::InvalidMaxDrawdown);
        require!(cooldown_slots >= 100, GuardError::InvalidCooldownSlots);

        let guard = &mut ctx.accounts.guard_config;
        guard.authority = ctx.accounts.authority.key();
        guard.token_mint = ctx.accounts.token_mint.key();
        guard.vault_account = ctx.accounts.vault_account.key();
        guard.max_drawdown_bps = max_drawdown_bps;
        guard.cooldown_slots = cooldown_slots;
        guard.twap_window_seconds = twap_window_seconds;
        guard.is_paused = false;
        guard.last_halt_slot = 0;
        guard.total_protected_volume = 0;
        guard.bump = ctx.bumps.guard_config;

        emit!(GuardInitializedEvent {
            authority: guard.authority,
            max_drawdown_bps,
            cooldown_slots,
        });

        Ok(())
    }

    pub fn assess_liquidity_health(
        ctx: Context<AssessLiquidity>,
        spot_price: u64,
        oracle_twap_price: u64,
    ) -> Result<()> {
        let guard = &mut ctx.accounts.guard_config;
        require!(!guard.is_paused, GuardError::VaultAlreadyPaused);

        let price_diff = if spot_price > oracle_twap_price {
            spot_price.saturating_sub(oracle_twap_price)
        } else {
            oracle_twap_price.saturating_sub(spot_price)
        };

        let deviation_bps = (price_diff as u128)
            .saturating_mul(10000)
            .checked_div(oracle_twap_price as u128)
            .unwrap_or(0) as u16;

        if deviation_bps > guard.max_drawdown_bps {
            guard.is_paused = true;
            guard.last_halt_slot = Clock::get()?.slot;

            emit!(CircuitBreakerTrippedEvent {
                vault: guard.vault_account,
                deviation_bps,
                halt_slot: guard.last_halt_slot,
            });
        }

        Ok(())
    }

    pub fn resume_liquidity(ctx: Context<ResumeLiquidity>) -> Result<()> {
        let guard = &mut ctx.accounts.guard_config;
        let current_slot = Clock::get()?.slot;

        require!(guard.is_paused, GuardError::VaultNotPaused);
        require!(
            current_slot >= guard.last_halt_slot.saturating_add(guard.cooldown_slots),
            GuardError::CooldownStillActive
        );

        guard.is_paused = false;

        emit!(CircuitResumedEvent {
            vault: guard.vault_account,
            resumed_slot: current_slot,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeGuard<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + GuardConfig::LEN,
        seeds = [b"guard-vault", authority.key().as_ref()],
        bump
    )]
    pub guard_config: Account<'info, GuardConfig>,
    pub token_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = authority,
        token::mint = token_mint,
        token::authority = guard_config,
        seeds = [b"guard-token-vault", guard_config.key().as_ref()],
        bump
    )]
    pub vault_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct AssessLiquidity<'info> {
    #[account(mut, has_one = authority)]
    pub guard_config: Account<'info, GuardConfig>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ResumeLiquidity<'info> {
    #[account(mut, has_one = authority)]
    pub guard_config: Account<'info, GuardConfig>,
    pub authority: Signer<'info>,
}

#[account]
pub struct GuardConfig {
    pub authority: Pubkey,
    pub token_mint: Pubkey,
    pub vault_account: Pubkey,
    pub max_drawdown_bps: u16,
    pub cooldown_slots: u64,
    pub twap_window_seconds: u32,
    pub is_paused: bool,
    pub last_halt_slot: u64,
    pub total_protected_volume: u64,
    pub bump: u8,
}

impl GuardConfig {
    pub const LEN: usize = 32 + 32 + 32 + 2 + 8 + 4 + 1 + 8 + 8 + 1;
}

#[event]
pub struct GuardInitializedEvent {
    pub authority: Pubkey,
    pub max_drawdown_bps: u16,
    pub cooldown_slots: u64,
}

#[event]
pub struct CircuitBreakerTrippedEvent {
    pub vault: Pubkey,
    pub deviation_bps: u16,
    pub halt_slot: u64,
}

#[event]
pub struct CircuitResumedEvent {
    pub vault: Pubkey,
    pub resumed_slot: u64,
}

#[error_code]
pub enum GuardError {
    #[msg("Max drawdown basis points exceeds safe protocol threshold of 5000 bps (50%).")]
    InvalidMaxDrawdown,
    #[msg("Cooldown period must be at least 100 slots to prevent oscillation.")]
    InvalidCooldownSlots,
    #[msg("Liquidity guard vault is already halted by circuit breaker.")]
    VaultAlreadyPaused,
    #[msg("Liquidity guard vault is not currently paused.")]
    VaultNotPaused,
    #[msg("Circuit recovery cooldown period has not elapsed yet.")]
    CooldownStillActive,
}
