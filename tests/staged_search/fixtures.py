"""Synthetic-only shared native fixture. Labels never claim model quality."""
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parent
def articles():
    rows = json.loads((ROOT.parent/'topic_search/corpus.json').read_text(encoding='utf-8'))['articles']
    rows += json.loads((ROOT/'corpus_extra.json').read_text(encoding='utf-8'))['articles']
    for i in range(40):
        rows.append(dict(id=300+i, title=f'Home club wins the away match {i}' if i%2==0 else f'高温烤箱让面包表皮更酥脆 {i}', summary='A sports match or baking report.', body='The home supporters celebrated three goals.' if i%2==0 else '面包师记录面团发酵时间和含水量，烘焙最后十分钟降低温度。', days=1, starred=False, feed=1 if i%2==0 else 2, topics=[]))
    return rows
