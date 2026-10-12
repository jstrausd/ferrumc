#!/usr/bin/env python3
"""Offline literal specification assembler. Never run by Rust acceptance tests.
No FerrumC code, Rust codecs, database output or production algorithm is imported.
Field layout comes from the accepted FCTR1/FCBQ1/FCBR1 contracts.
"""
from pathlib import Path
import struct,hashlib,json
HERE=Path(__file__).resolve().parent
u16=lambda n:struct.pack('>H',n)
u32=lambda n:struct.pack('>I',n)
u64=lambda n:struct.pack('>Q',n)
i32=lambda n:struct.pack('>i',n)
SCOPE=u32(17)+u32(3)
IDENTITY=b'FCGI'+u16(1)+u16(1)+u16(21)+u16(8)+u16(1)+u32(1)+struct.pack('>q',-827)+b'\0'
PLAYER=bytes([0x8d])*16
op=lambda n:bytes([n])*16
stamp=lambda tag,n:bytes([tag])+u64(n)
FRESH=stamp(0,0)
def stack(n):
 return b'FCIS\0\1\1\0\x0fminecraft:stone'+bytes([n,2,1])+bytes.fromhex('02000000030000000400000007070000000c00000004dead00ff00000004')
def player(n):
 b=b'FCPS\0\3'+struct.pack('>dddffBH',-15.25,64.5,-0.0,17.5,-0.0,7,46)
 for slot in range(47):
  s=stack(n if slot==7 else slot%63+1);b+=u32(len(s))+s
 return b
def envelope(n):return u32(3)+b'\0'+player(n)
def chest(n):return b'\x7f\0\1\1'+b''.join(u32(len(s))+s for s in [stack(n if i==0 else i+1) for i in range(27)])
def key(x):return SCOPE+i32(x)+i32(-1)
def pos(x,y):return i32(x)+i32(y)+i32(-1)
def full(x,schema,changes=(),entities=()):
 changes=dict(changes);b=u32(schema)+i32(x)+i32(-1)+b'FCHK'+u32(2)+bytes([24])
 for section in range(24):
  floor=-64+16*section
  if any(floor<=p[1]<floor+16 for p in changes):
   b+=b'\1'+b''.join(u32(changes.get((bx,y,bz),0)) for y in range(floor,floor+16) for bz in range(-16,0) for bx in range(x*16,x*16+16))
  else:b+=b'\0'
 return b+u32(len(entities))+b''.join(pos(*p[:2])+u32(len(v))+v for p,v in entities)
def blob(v):return (b'\0'+u32(0)) if v is None else b'\1'+u32(len(v))+v
def pair(f=None,o=None):return blob(f)+blob(o)
def effect(x,y,old,new,entity=None):return u32(97)+u64(481)+b'\1'+PLAYER+b'\0'+pos(x,y)+u32(old)+u32(new)+blob(None)+blob(entity)
def q(kind,n,before_player,after_player,entries,guards=(),effects=()):
 active=kind==1
 b=b'FCBQ'+u16(1)+bytes([kind])+op(n)+SCOPE+IDENTITY+(PLAYER if active else bytes(16))+(stamp(1,1) if active else FRESH)+blob(before_player)+blob(after_player)+bytes([len(entries)])
 for x,before_stamp,before,action,after in entries:b+=key(x)+before_stamp+before+bytes([action])+after
 return b+u16(len(guards))+b''.join(pos(*p) for p in guards)+u16(len(effects))+b''.join(u32(len(e))+e for e in effects)
def receipt(body,player_stamp,chunks,first=None,count=0):
 return b'FCBR'+u16(1)+u32(len(body))+body+player_stamp+bytes([len(chunks)])+b''.join(chunks)+(b'\0'+u64(0)+u16(0) if first is None else b'\1'+u64(first)+u16(count))+hashlib.sha256(b'FerrumC block operation v1\0'+body).digest()+bytes(8)
# Standalone v1 Player bootstrap, inactive Entity fields are exact zero/Fresh.
fctr_q=bytes([2])+op(1)+SCOPE+IDENTITY+PLAYER+bytes(16)+FRESH+FRESH+b'\1'+u64(len(envelope(11)))+envelope(11)+b'\0'+u64(0)
fctr=bytearray(256);fctr[:7]=b'FCTR'+u16(1)+b'\2';fctr[7:23]=op(1);fctr[23:55]=hashlib.sha256(b'FerrumC ownership request v1\0'+fctr_q).digest();fctr[55:63]=SCOPE;fctr[63:90]=IDENTITY;fctr[90:106]=PLAYER;fctr[131:140]=stamp(1,1)
tables=['meta','generation','chunk','chunk_overlay','entity','player','plugin','mutation_log','mutation_batch','entity_locator','entity_chunk','entity_chunk_epoch','player_ownership','entity_ownership','ownership_receipt','chunk_ownership','scoped_mutation_log']
rows={f'ferrumc:{t}':{} for t in tables}
meta={'format_version':7,'ownership_request_limit':2*1024*1024,'ownership_receipt_limit':1000,'ownership_revision_limit':1000,'ownership_receipt_count':2,'ownership_revision_count':1,'block_request_limit':32*1024*1024,'block_revision_limit':1000,'block_revision_count':2,'operation_receipt_byte_limit':256*1024*1024,'scoped_journal_row_limit':10000,'scoped_journal_byte_limit':64*1024*1024,'scoped_journal_count':0,'scoped_journal_bytes':0}
seed=q(3,2,None,None,[(x,FRESH,pair(),1,pair(full(x,55))) for x in range(2)])
seed_receipt=receipt(seed,FRESH,[stamp(1,1)]*2)
meta['operation_receipt_bytes']=272+16+len(seed_receipt)
rows['ferrumc:meta']={k.encode():u64(v) for k,v in meta.items()}
rows['ferrumc:generation'][SCOPE]=IDENTITY
rows['ferrumc:player'][PLAYER]=envelope(11)
rows['ferrumc:player_ownership'][PLAYER]=b'FCOP'+u16(1)+b'\1'+SCOPE+u64(1)+op(1)
rows['ferrumc:ownership_receipt'][op(1)]=bytes(fctr)
rows['ferrumc:ownership_receipt'][op(2)]=seed_receipt
for x in range(2):
 rows['ferrumc:chunk'][key(x)]=full(x,55)
 rows['ferrumc:chunk_ownership'][key(x)]=b'FCOB'+u16(1)+stamp(1,1)+op(2)
def save_rows(name):
 (HERE/name).write_text(''.join(f'{table}\t-\t-\n'+''.join(f'{k.hex()}\t{v.hex()}\n' for k,v in sorted(data.items())) for table,data in sorted(rows.items())))
save_rows('before17.tsv')
post=[full(0,57,[((15,64,-1),100),((15,65,-1),101),((14,64,-1),200)],[((14,64,-1),chest(8))]),full(1,56,[((16,64,-1),300)])]
effects=[effect(15,64,0,100),effect(15,65,0,101),effect(14,64,0,200,chest(8)),effect(16,64,0,300)]
joint=q(1,3,envelope(11),envelope(10),[(x,stamp(1,1),pair(full(x,55)),1,pair(post[x])) for x in range(2)],[(15,63),(16,65)],effects)
joint_receipt=receipt(joint,stamp(1,2),[stamp(1,2)]*2,0,4)
for x in range(2):
 rows['ferrumc:chunk'][key(x)]=post[x];rows['ferrumc:chunk_ownership'][key(x)]=b'FCOB'+u16(1)+stamp(1,2)+op(3)
rows['ferrumc:player'][PLAYER]=envelope(10);rows['ferrumc:player_ownership'][PLAYER]=b'FCOP'+u16(1)+b'\1'+SCOPE+u64(2)+op(3)
rows['ferrumc:ownership_receipt'][op(3)]=joint_receipt
for ordinal,e in enumerate(effects):rows['ferrumc:scoped_mutation_log'][u64(ordinal)]=b'FCBL'+u16(1)+SCOPE+op(3)+u16(ordinal)+u64(ordinal)+e
for k,v in {'ownership_receipt_count':3,'operation_receipt_bytes':meta['operation_receipt_bytes']+16+len(joint_receipt),'scoped_journal_count':4,'scoped_journal_bytes':sum(8+len(v) for v in rows['ferrumc:scoped_mutation_log'].values()),'last_scoped_mutation_id':3}.items():rows['ferrumc:meta'][k.encode()]=u64(v)
save_rows('after17.tsv')
for name,data in {'stack8.fcis':stack(8),'before.fcps3':player(11),'after.fcps3':player(10),'chest8.entity':chest(8),'seed.fcbq':seed,'seed.fcbr':seed_receipt,'joint.fcbq':joint,'joint.fcbr':joint_receipt}.items():(HERE/name).write_bytes(data)
files={p.name:{'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in HERE.iterdir() if p.is_file() and p.name not in ['fixture.freeze.json']}
(HERE/'fixture.freeze.json').write_text(json.dumps({'kind':'fixed independent contract fixtures, no runtime output','files':files},indent=2)+'\n')
