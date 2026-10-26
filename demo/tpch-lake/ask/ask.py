"""Ask the lake a question from the voice and gesture layer.

    uv run --with snowflake-connector-python ask/ask.py "Which region earned the most in 1997?" [TABLE]

Prints the answer as one JSON line: {"question", "focus_table", "answer"}.
Uses the connection named by SNOWFLAKE_CONNECTION (default "filagree") in ~/.snowflake/connections.toml.
"""

import json
import os
import sys

import snowflake.connector


def ask(question: str, focus_table: str | None = None) -> str:
    with snowflake.connector.connect(
        connection_name=os.environ.get("SNOWFLAKE_CONNECTION", "filagree"),
        warehouse="FILAGREE_WH",
    ) as conn:
        cur = conn.cursor()
        cur.execute("CALL FILAGREE.AI.ASK_LAKE(%s, %s)", (question, focus_table))
        return cur.fetchone()[0]


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    question = sys.argv[1]
    focus = sys.argv[2] if len(sys.argv) > 2 else None
    print(json.dumps({"question": question, "focus_table": focus, "answer": ask(question, focus)}))
