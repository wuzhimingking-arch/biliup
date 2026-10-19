"""Isolated hook regression: synthetic video and fixture XML are not live acceptance evidence."""
import pathlib,tempfile,subprocess,shutil,hashlib,os
image=os.environ.get('BILIUP_TEST_IMAGE','biliup-danmaku-fix:test')
source=pathlib.Path(__file__).parent
with tempfile.TemporaryDirectory(prefix='biliup-hook-test-') as temp:
 root=pathlib.Path(temp)
 for name in ['burn-hook-safe.sh','burn-hook.sh']:shutil.copy2(source/name,root/name)
 base=['docker','run','--rm','-i','--network','none','--mount','type=bind,source='+str(root)+',target=/opt']
 subprocess.run(base+['--entrypoint','ffmpeg',image,'-nostdin','-v','error','-f','lavfi','-i','color=s=160x90:r=25:d=2','-c:v','libx264','-f','flv','/opt/video.flv'],check=True)
 original=hashlib.sha256((root/'video.flv').read_bytes()).digest()
 real_hook=(root/'burn-hook.sh').read_bytes()
 (root/'burn-hook.sh').write_text('#!/bin/sh\ntouch /opt/called\nexit 70\n')
 def run(payload):return subprocess.run(base+['--entrypoint','sh',image,'/opt/burn-hook-safe.sh'],input=payload,text=True,capture_output=True,timeout=100)
 for payload,xml,called in [('/opt/video.flv\n\n',None,False),('/opt/video.flv\n/opt/chat.xml\n','<i></i>',False),('/opt/video.flv\n/opt/chat.xml\n','<i><d p="0,1,25,16777215,0,0,0,0">FIXTURE ONLY</d></i>',True)]:
  (root/'called').unlink(missing_ok=True)
  if xml is not None:(root/'chat.xml').write_text(xml)
  result=run(payload);assert result.returncode==0,result.stderr
  assert (root/'called').exists()==called
  assert hashlib.sha256((root/'video.flv').read_bytes()).digest()==original
 (root/'bad.flv').write_bytes(b'not a video')
 assert run('/opt/bad.flv\n\n').returncode!=0
 (root/'burn-hook.sh').write_text('#!/bin/sh\nexit 71\n')
 assert run('/opt/video.flv\n/opt/chat.xml\n').returncode==71
 (root/'burn-hook.sh').write_bytes(real_hook)
 (root/'stubs').mkdir();(root/'stubs/sleep').write_text('#!/bin/sh\nexit 0\n');(root/'stubs/sleep').chmod(0o755)
 result=subprocess.run(base+['-e','PATH=/opt/stubs:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin','--entrypoint','sh',image,'/opt/burn-hook.sh'],input='/opt/video.flv\n/opt/chat.xml\n',text=True,capture_output=True,timeout=15)
 assert result.returncode==72,result.stderr
 assert not list((root/'.burn-queue/pending').glob('*.job'))
 assert len(list((root/'.burn-queue/results').glob('*.cancelled')))==1
 assert hashlib.sha256((root/'video.flv').read_bytes()).digest()==original
 print('PASS: missing/empty XML, failed burn fallback, corrupt video blocking, and unclaimed queue withdrawal; original preserved')
