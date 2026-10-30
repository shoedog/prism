"""Byte-real ae49d937 producer fixtures (local paths in diagnostic lines replaced by placeholders); raw captures and SHA receipts live outside repo."""
import base64
import os
import tempfile
import unittest
import zlib
from pathlib import Path
import memreport_idle as idle

CAPTURES = {'stdio': 'eNrtmN2ymzYQx+/7FDxAU+sTpMxwlYvc9g0YAeJYE5AIEm7y9lkgOIAb+5y2xyZpPB4btEha9vdffbWd8c2bpmjfRqbUNpjwOcUFkxzllJWJjlpTpoShhETG+qBsoTP460KKE0kEfHlMJKMCwSfSrSuOmfVrI0fJYGycdaONxJTSSFeVLoI56azTtdFVWqna60ifwIt07KJvo04r7+z5VoWgmzakKHrSVncqGDCiqHC2Mk99p8vMlDU4qAufkj+GTl3udXdaGawLmWrb2hQqr3VU9lM7g2sbU9E+pb39YN1fNqqdKrNWdV4PD86lVXZStSnPDczlxpb607Igdy740Kl2Wegh5IXqtt16/bHXEOgUw/vXEAFwP/8ctE+/BrEZrlwfCtfoFH7aWgf9W3sHlJLjJUqeIInj6yjz3tTl1NGMs9SNsmXmumwG+zKCc/yCaTS00LRDRFZ+rqiib3dj5GYV4Y2KHkybvIi2VaeoUMVRv43e/fl+uowa4z14Ex1C0x4mPRhUkMNofeNDadxUfIDqh6U4eEIURWWBq1iWBCFZiCIvK4LgESVzSGGJieSUJkQInVPFK/I7ODzSNfbp9dVHqSRULtXHmOCCcPQc/VXGGn+8kwDB1X8qwJHgWn2McClFwi/kR/A4kJ7VhylHiJG1+hhUTwgSN+VHvy8/xh4w2gy8WbLhHQ+fZ/B+EGgItkgYTla8IXq7BM5+EuBtnw/vNgTpF/YN9skBXX6jzvdHndM1dY55jAm/Tr1WPmRfX2XGHpyrs9n13Qzpj1xTxFdo8/vTFpQJJldLSII4QxKq3eDd2yF+M+mB238KePBsBZgnfI+MLxM6+T7iIbaPgcwJ3ULGJJFcXIdcGt86r+r1duH1Yf8g2Sx2iZqjC9Sc3Zy1z6i1vWNWkxhMPwZsuUfYMZZb2IQwIvF12FPR/TCznTNWA7Is15Xr9CJpYI8921QVdLcIMYZ8Wle7mo2rVq6iPGpweWoypTzmRExFY9W5pIKXA3nZMD+JY0yFFGxhmSqAQQhM48UhFtqfkGNCxGYVEicCZq1bQr77Ydbg67OmKrKv0Qvja9Tj5DHUJSMb6gKmKva8I8x7HiH9G+xHEzbICcXD+npLHEMSQPEZOMHj/Yo3ZMqQFDdxk6u4xaNwiwvcsDJN+M3ZarHVgKEUeraq9UcXXhdyDNteJHZOmv5EpP/m6Oj/jPtyc4nZS2l/AU131O8=', 'shared': 'eNrlmUtz2zYQgO/9Fbj1Ekd4P9LRqYde+w84IAFamFAAS0BO8u+7pExHlGzZSaexLGs0EoklsIvdD4sH+yHk7c226ZGzfpviJxScjyWUb2tMLCXM1LzFsqHWE4MJk9K21nOtNHHWONHgRlBnpSJWNK0WLVZWC7hTDvXBrSnHSqEQc7Gx8RX8DWVNlKEavkLj+YN8n5pNFfOBkEltOBmFYFiaZERJrQTybeubEu58Nfgu+Hbd2i575O/A9PWkY9ejwduc4sOtLcVv+7LG6NZHP9gSQIhRk2IbbneDd1VwHVjom7ymH0etqc5+uFsIYiqV7fsuNLbuPHK7fTujbUeipr9d7+LnmL5E1CXrqt4O2Y8PzqVtdWe74B4amMtDdP7rYUGdUsllsP1hYYY4NXY4Vpv9PzsPnl4T6H8HHgDz62/F5/W9F7fjVdqVJm39Gn76zhf/W3+CAQQP7YM3OSGjYxxQmwa0Ktt+ta8ccENXbfhawJc3A4QTPH4oH8rqQYtbHbf2Mafm8yNmXA6NHAsmFzQyJYygAp/Hsd6Fzu1VzUg6v7XRVWmoZjgfQQ1PDJ7Bs4Sth/rbfozpws4Fl/iBfHJE/isTSn+I0GjvUGObjf+E/vz7r/0l2oacwZoTDCfpTd5Y8Nu+fAX1V6ThRuCacae8UNQy7BrSSuMoxqbRTe1aiuERa2qBlSHUCMYU1drXzIqWfgCLp3CGeHvZrHJOGD5kVUkmucLmPKshhhJsV02d/DW0cr6glWklDQU7z0A7RX1JLFOKiVHNMbJECMzpd2IVJpyoJbBMSwrhkc8Sy54mlumXJNW3T0i/q0e3jO5953zs9Xv3HQ9+XXhQc4wH04y/ZLJrIY/kzavwgS8ycYirIoMJuSRDGPxs4uhsLtW9F2YySkpdNff61ah4zTWQvBYwJOZcQs44BINipSWjUqrzaOzi6PoZijHEj7EwBpz+MAuTXcsZhI4j9hJ4OJ1B1NM4wMKUvD0guDwBQkounskVLuQ+ZViHLnZN/zcYF5sk9LVRofAJFQZLKl5IhY+/LFlQTg29VC7MlXEhCDnmQjEiYBV2lot90bvNEnYMblX7Ng3+wG1K4Flm2+KHWaSxGifkZbWz8Vi08sSTehzSaOPB5H2TsHzBgMO+aKo6l7TQOQAxlvlJIinUFvJAsq9AJCOw19MHh5r4qpBXQuplKiQQIWPUZZ0pTpY+MQjoZaVEQs4BovBbA0RyyY4A0VIz/IKceLCkhhEMmqPt8yaVJ5Ij+6nkqJbJUWlqhDkHxyaUIzAomTbWJ/tvZsbThgcsIKGNb34WVAhpRnXPQkHPQkHeCxSPnN29JzJOt1uEXR0Y5BQM/MLp5Jed2p3SgS8uZfD/ToaNKPVjV9KAbP4M4JVNyPdiVBLKm11BDqa1PxB00Q0W4o4SqEcxoQaiFAuCCp1vy/kXtCX1vXcLjb/Pim5GLaOSD+hLKJuDpqd2/wUeK9Fs'}

def fixture(mode):
    return zlib.decompress(base64.b64decode(CAPTURES[mode]))

def instance(mode):
    line = fixture(mode).splitlines()[0].decode()
    fields = dict(token.split('=', 1) for token in line.split() if '=' in token)
    return {k: fields[k] for k in ('pid', 'instance_start', 'identity')}

class Collector(unittest.TestCase):
    def test_idle_collector_visibility(self):
        with tempfile.TemporaryDirectory() as tmp:
            paths = []
            for mode in ('stdio', 'shared'):
                path = Path(tmp) / mode
                path.write_bytes(fixture(mode))
                paths.append({'path': str(path), 'instance': instance(mode), 'role': mode})
            c = idle.Collector(paths)
            c.poll(); c.poll(); c.finish()
            self.assertEqual(c.reasons, [])
            self.assertEqual(len([e for e in c.events if e['event'] == 'publication']), 4)
            self.assertEqual({e['sink'] for e in c.events}, {str(p['path']) for p in paths})
            self.assertTrue(all('offset' in e and 'file_id' in e for e in c.events))
            path = Path(tmp) / 'stdio'
            path.write_bytes(fixture('stdio')[:12])
            c.poll(); c.finish()
            self.assertIn('truncated_stream', c.reasons)
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / 'x'
            line = fixture('shared').splitlines(keepends=True)[0]
            cases = [(line + line, 'duplicate_sequence'),
                     (line.replace(b'sequence=1', b'sequence=2'), 'sequence_gap'),
                     (line.replace(b' mono_ns=', b' lost_ns='), 'malformed_event'),
                     (line[:-1], 'partial_line'),
                     (line.replace(b'outcome=complete', b'outcome=' + b'x' * 700), 'possibly_capped'),
                     (b'prism-mcp daemon: the log reached its cap of 8 MiB; later lines are dropped\n', 'log_cap')]
            for raw, reason in cases:
                p.write_bytes(raw)
                c = idle.Collector([{'path': str(p), 'instance': instance('shared')}])
                c.poll(); c.finish()
                self.assertIn(reason, c.reasons)
            p.write_bytes(line)
            wrong = dict(instance('shared'), pid='1')
            c = idle.Collector([{'path': str(p), 'instance': wrong}]); c.poll()
            self.assertIn('wrong_instance', c.reasons)
            restart = dict(instance('shared'), instance_start='1')
            c = idle.Collector([{'path': str(p), 'instance': restart}]); c.poll()
            self.assertIn('wrong_instance', c.reasons)
            c = idle.Collector([{'path': str(p)+'missing', 'instance': instance('shared')}])
            c.poll(); c.finish()
            self.assertIn('missing_stream', c.reasons)

    def test_cap_boundary_and_mirrored_sink(self):
        line = fixture('stdio').splitlines(keepends=True)[0]
        body = line[len(b'prism-mcp: '):-1].replace(b' sequence=1', b'')
        prefix = line[:line.index(b'outcome=')+8]
        for size, capped in ((966, False), (967, True)):
            raw = prefix + b'x' * (size - len(body) + len(b'complete')) + b'\n'
            event, reason = idle.parse_line(raw)
            self.assertEqual(reason == 'possibly_capped', capped)
        with tempfile.TemporaryDirectory() as tmp:
            a, b = Path(tmp)/'a', Path(tmp)/'b'
            a.write_bytes(line); os.link(a, b)
            c = idle.Collector([{'path': str(p), 'instance': instance('stdio')} for p in (a,b)])
            c.poll(); c.finish()
            self.assertEqual(len(c.events), 1)
            self.assertEqual(c.reasons, [])

    def test_malformed_numeric_and_replaced_file(self):
        line = fixture('stdio').splitlines(keepends=True)[0]
        for raw in (line.replace(b'epoch_ns=', b'epoch_ns=x'), line.replace(b'pid=', b'pid=-'),
                    line.replace(b'cpg=unknown', b'cpg=unknown pid=1')):
            self.assertEqual(idle.parse_line(raw)[1], 'malformed_event')
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp)/'x'; p.write_bytes(line)
            c = idle.Collector([{'path':str(p),'instance':instance('stdio')}]); c.poll()
            other = Path(tmp)/'y'; other.write_bytes(line); other.replace(p); c.poll()
            self.assertIn('replaced_stream', c.reasons)

    def test_truncate_and_regrow(self):
        line=fixture('stdio').splitlines(keepends=True)[0]
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'log'; p.write_bytes(line)
            c=idle.Collector([{'path':str(p),'instance':instance('stdio')}]); c.poll()
            p.write_bytes(line.replace(b'event=startup',b'event=changed'))
            c.poll()
            self.assertIn('truncated_stream',c.reasons)

if __name__ == '__main__':
    unittest.main()
