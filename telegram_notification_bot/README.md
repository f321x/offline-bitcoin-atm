# Telegram Notification Bot

A simple Telegram bot that monitors your ATM's LNBits wallet and sends you notifications when:

- Someone makes a withdrawal from the ATM
- The wallet balance drops below a configured threshold

## Prerequisites

- A server or computer that stays online 24/7 (e.g. a Raspberry Pi, VPS, or home server)
- A Telegram account

## Setup

### 1. Create a Telegram Bot

1. Open Telegram and search for **@BotFather**
2. Send `/newbot` and follow the prompts to choose a name and username
3. BotFather will reply with a **bot token** (looks like `123456789:ABCdefGhIJKlmNoPQRsTUVwxYZ`) -- save this

### 2. Get Your Telegram Chat ID

1. Search for **@userinfobot** (or any "chat ID" bot) on Telegram
2. Send it any message -- it will reply with your **chat ID** (a number like `123456789`)

### 3. Get Your LNBits API Key

1. Log in to your LNBits instance
2. Open the wallet connected to your ATM
3. Copy the **read-only API key** (not the admin key)

### 4. Configure the Bot

Open `main.py` and fill in the configuration variables at the top of the file:

```python
url = "https://legend.lnbits.com/api/v1/wallet"  # your LNBits instance URL
api_key = "ABCDEFG"                                # your LNBits read-only API key
bot_token = "123:ABC"                              # your Telegram bot token from BotFather
chat_id = "123456789"                              # your Telegram chat ID

min_balance = 1000      # notify when balance drops below this many sats
refresh_interval = 5600  # how often to check the balance, in seconds
```

### 5. Run the Bot

```bash
python3 main.py
```

The bot will run in an infinite loop, checking the wallet balance every `refresh_interval` seconds.

To keep it running after closing your terminal, you can use `screen`, `tmux`, or create a systemd service:

```bash
# Quick option: run in background with screen
screen -S atmbot
python3 main.py
# Press Ctrl+A, then D to detach. Reattach later with: screen -r atmbot
```
