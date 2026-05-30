"""#147 OSS参照採点 (MahjongRepository/mahjong)。id|han|fu|yaku を出力。"""
from mahjong.hand_calculating.hand import HandCalculator
from mahjong.tile import TilesConverter
from mahjong.hand_calculating.hand_config import HandConfig, OptionalRules
from mahjong.constants import EAST, SOUTH, WEST, NORTH
calc = HandCalculator()
WIND = {'E': EAST, 'S': SOUTH, 'W': WEST, 'N': NORTH}

# (id, man, pin, sou, honors, win(suit,val), tsumo, riichi, dealer, round, seat)
HANDS = [
 ("pinfu_tsumo",      "234","567","23456799","",   ("s","7"),True, False,False,"E","S"),
 ("pinfu_ron",        "234","567","23456799","",   ("s","7"),False,False,False,"E","S"),
 ("tanyao_ron",       "234345","678","23455","",   ("s","4"),False,False,False,"E","S"),
 ("sanshoku_ron",     "234","23467899","234","",   ("p","8"),False,False,False,"E","S"),
 ("ittsu_ron",        "123456789","23499","","",   ("p","4"),False,False,False,"E","S"),
 ("iipeikou_ron",     "223344","567","23499","",   ("s","4"),False,False,False,"E","S"),
 ("chiitoi_ron",      "1199","2288","5566","77",   ("z","7"),False,False,False,"E","S"),
 ("yakuhai_haku_ron", "234","234","234","55511",   ("p","2"),False,False,False,"E","S"),
 ("kanchan_riichi_ron","12399","234567","234","",  ("m","2"),False,True, False,"E","S"),
 ("tanki_riichi_ron", "234","234567","234","44",   ("z","4"),False,True, False,"E","S"),
 ("ankou_term_tsumo", "111","234567","23499","",   ("s","4"),True, False,False,"E","S"),
 ("toitoi_sanankou_ron","11199","333","555","777", ("z","7"),False,False,False,"E","S"),
 ("chinitsu_ron",     "2342345676 7899".replace(" ",""),"","","",("m","9"),False,False,False,"E","S"),
 ("honitsu_ittsu_ron","12345678999","","","111",   ("m","9"),False,False,False,"E","S"),
 ("kokushi_ron",      "19","19","19","12345677",   ("z","7"),False,False,False,"E","S"),
 ("suuankou_tsumo",   "11199","333","555","777",   ("z","7"),True, False,False,"E","S"),
 ("daisangen_ron",    "234","99","","555666777",   ("m","2"),False,False,False,"E","S"),
 ("chinroutou_tsumo", "111999","111999","11","",   ("s","1"),True, False,False,"E","S"),
 ("tsuuiisou_daisan_ron","","","","11155566677722",("z","2"),False,False,False,"E","S"),
 ("dealer_haneman_tsumo","234","234","23423499","",("s","4"),True, True, True, "E","E"),
]

for hid,m,p,s,h,win,tsumo,riichi,dealer,rnd,seat in HANDS:
    tiles = TilesConverter.string_to_136_array(man=m,pin=p,sou=s,honors=h)
    ws,wv = win
    kw = {'m':'man','p':'pin','s':'sou','z':'honors'}[ws]
    wt = TilesConverter.string_to_136_array(**{kw:wv})[0]
    cfg = HandConfig(is_tsumo=tsumo,is_riichi=riichi,player_wind=WIND[seat],round_wind=WIND[rnd],
                     options=OptionalRules(has_open_tanyao=True))
    res = calc.estimate_hand_value(tiles, wt, config=cfg)
    if res.han is None:
        print(f"{hid}|INVALID:{res.error}|n={len(tiles)}"); continue
    yaku = ",".join(sorted(str(y) for y in (res.yaku or [])))
    c = res.cost or {}
    print(f"{hid}|han={res.han}|fu={res.fu}|main={c.get('main')}|add={c.get('additional')}|yaku={yaku}")
