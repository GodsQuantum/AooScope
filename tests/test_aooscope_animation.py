import json
import tempfile
import unittest
from pathlib import Path

from aooscope.animation_runtime import animation_sensor_key, active_animation_keys, phase_value
from aooscope.media_library import MediaLibrary
from aooscope.page_compiler import compile_page


class AnimationTests(unittest.TestCase):
    def test_sensor_key_is_stable_and_safe(self):
        self.assertEqual(animation_sensor_key('orbit logo #1'), 'aooscope_animation_orbit_logo__1_phase')

    def test_phase_wraps_to_0_100(self):
        self.assertEqual(phase_value(0.0, 4.0), 0)
        self.assertEqual(phase_value(2.0, 4.0), 50)
        self.assertEqual(phase_value(4.5, 4.0), 12.5)

    def test_compiler_emits_rotating_partial_update_visual(self):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); lib=MediaLibrary(root)
            page={'id':'logo','name':'Logo','enabled':True,'duration':8,'background':{'color':'#071019'},'layers':[
                {'id':'orb','type':'animation','animation':'orbit','x':200,'y':40,'width':300,'height':280,'z':2,'opacity':1,'color':'#35d9ff','speed_seconds':4}
            ]}
            result=compile_page(page,{},lib,root/'out')
            sensor=result.monitor_panel['sensor'][0]
            self.assertEqual(sensor['mode'],2)
            self.assertEqual(sensor['label'], animation_sensor_key('orb'))
            self.assertEqual(sensor['minAngle'],0)
            self.assertEqual(sensor['maxAngle'],360)
            self.assertTrue((root/'out'/sensor['pic']).is_file())

    def test_runtime_reads_only_promoted_animation_layers(self):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); rev=root/'compiled'/'r1'; rev.mkdir(parents=True)
            (root/'compiled'/'current.json').write_text(json.dumps({'revision_id':'r1'}))
            (rev/'source-pages.json').write_text(json.dumps({'carousel':['p'],'pages':{'p':{'enabled':True,'layers':[{'id':'orb','type':'animation','animation':'orbit'}]}}}))
            self.assertEqual(active_animation_keys(root), ['aooscope_animation_orb_phase'])

if __name__=='__main__': unittest.main()
