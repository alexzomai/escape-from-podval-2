"""Эталонный прогон оригинальной игры на Python/pygame для теста tests/parity.rs.

Запускает исходники оригинала с ботом-автопилотом и записывает по каждому кадру
ввод и CRC32 экрана, состояние игры — каждые STATE_EVERY кадров, скриншоты —
каждые SAVE_EVERY кадров, а также экраны меню, победы и поражения.

    python harness.py <папка с оригиналом> <папка вывода> [кадров]

Нужны pygame и Pillow (Pillow грузит картинки, если pygame собран без SDL_image).
"""
import math
import os
import sys
import zlib

os.environ['SDL_VIDEODRIVER'] = 'dummy'
os.environ['SDL_AUDIODRIVER'] = 'dummy'

SRC = os.path.abspath(sys.argv[1])
OUT = os.path.abspath(sys.argv[2])
FRAMES = int(sys.argv[3]) if len(sys.argv) > 3 else 7200
SAVE_EVERY = int(os.environ.get('SAVE_EVERY', '1200'))
STATE_EVERY = int(os.environ.get('STATE_EVERY', '10'))
os.makedirs(os.path.join(OUT, 'frames'), exist_ok=True)
os.chdir(SRC)
sys.path.insert(0, SRC)

import pygame
from PIL import Image


def load(path):
    im = Image.open(path).convert('RGBA')
    return pygame.image.frombuffer(im.tobytes(), im.size, 'RGBA').copy()


pygame.image.load = load


class GameOver(Exception):
    pass


class FakeSound:
    plays = 0

    def __init__(self, *a):
        pass

    def set_volume(self, v):
        pass

    def play(self):
        FakeSound.plays += 1


class FakeMusic:
    def load(self, path):
        if 'win' in path or 'loss' in path:
            raise GameOver(path)

    def set_volume(self, v):
        pass

    def play(self, *a):
        pass

    def stop(self):
        pass


class FakeMixer:
    Sound = FakeSound
    music = FakeMusic()

    @staticmethod
    def pre_init(*a, **k):
        pass

    @staticmethod
    def init(*a, **k):
        pass


pygame.mixer = FakeMixer


class Flipped(Exception):
    pass


class Keys:
    def __init__(self, pressed):
        self.pressed = pressed

    def __getitem__(self, k):
        return k in self.pressed


class FakeClock:
    def tick(self, *a):
        return 0

    def get_fps(self):
        return 60.0


cur = {'keys': set(), 'dx': 0, 'click': False, 'mouse': (0, 0), 'pressed': (0, 0, 0)}


def events():
    ev = []
    if cur['click']:
        ev.append(pygame.event.Event(pygame.MOUSEBUTTONDOWN, button=1, pos=(0, 0)))
        cur['click'] = False
    return ev


pygame.key.get_pressed = lambda: Keys(cur['keys'])
pygame.event.get = lambda *a, **k: events()
pygame.mouse.get_focused = lambda: True
pygame.mouse.set_pos = lambda *a: None
pygame.mouse.set_visible = lambda *a: None
pygame.mouse.get_pressed = lambda *a: cur['pressed']

pygame.init()
from settings import *  # noqa: E402,F403
from mapdata import map_width, map_height, world_map  # noqa: E402
from sprites import Sprites  # noqa: E402
from player import Player  # noqa: E402
from ray_casting import ray_casting  # noqa: E402
from render import Rendering, main_menu  # noqa: E402
from interactions import Interaction, ray_casting_npc_player  # noqa: E402

screen = pygame.display.set_mode((WIDTH, HEIGHT), pygame.DOUBLEBUF)
minimap = pygame.Surface((MAP_TILE * map_width, MAP_TILE * map_height))
clock = FakeClock()


def save_png(name):
    data = pygame.image.tostring(screen, 'RGB')
    Image.frombytes('RGB', (WIDTH, HEIGHT), data).save(os.path.join(OUT, name))


# --- статичные экраны: меню (первый кадр), победа, поражение ---
real_flip = pygame.display.flip


def capture_flip(name):
    def flip():
        save_png(name)
        raise Flipped()
    return flip


SCREENS = [('none', (0, 0)), ('start', (HALF_WIDTH, HALF_HEIGHT)), ('exit', (HALF_WIDTH, HALF_HEIGHT + 200))]
# меню без наведения не сохраняем: кнопки в обычном виде есть на двух других кадрах
for tag, pos in SCREENS[1:]:
    pygame.mouse.get_pos = lambda pos=pos: pos
    pygame.display.flip = capture_flip(f'menu_{tag}.png')
    try:
        main_menu(screen, clock)
    except Flipped:
        pass

player = Player()
sprites = Sprites()
render = Rendering(screen, minimap, player, clock)
for tag, pos in SCREENS[::2]:
    pygame.mouse.get_pos = lambda pos=pos: pos
    for kind in ('win', 'lose'):
        pygame.display.flip = capture_flip(f'{kind}_{tag}.png')
        try:
            getattr(render, kind)()
        except Flipped:
            pass
pygame.display.flip = real_flip

interaction = Interaction(player, sprites, render)
pygame.mouse.get_pos = lambda: (HALF_WIDTH + cur['dx'], HALF_HEIGHT)


# --- бот ---
def passable(tx, ty):
    return 0 <= tx < map_width and 0 <= ty < map_height and (tx * TILE_SIZE, ty * TILE_SIZE) not in world_map


def bfs_path(start, goals):
    prev = {start: None}
    queue = [start]
    head = 0
    while head < len(queue):
        node = queue[head]
        head += 1
        if node in goals:
            path = []
            while node is not None:
                path.append(node)
                node = prev[node]
            return path[::-1]
        x, y = node
        for nxt in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
            if nxt not in prev and passable(*nxt):
                prev[nxt] = node
                queue.append(nxt)
    return None


def wrap(a):
    return (a + math.pi) % (2 * math.pi) - math.pi


bot = {'last': None, 'stuck': 0, 'unstick': 0}


def policy(frame):
    px, py = player.pos
    keys, dx, click = set(), 0, False
    alive = [s for s in sprites.sprite_list if not s.is_dead]
    seen = [s for s in alive if ray_casting_npc_player(s.x, s.y, world_map, player.pos)]
    if seen:
        t = min(seen, key=lambda s: (s.x - px) ** 2 + (s.y - py) ** 2)
        diff = wrap(math.atan2(t.y - py, t.x - px) - player.angle)
        dx = int(max(-45, min(45, diff / player.sensitivity)))
        click = abs(diff) < 0.04 and frame % 2 == 0
        phase = (frame // 50) % 4
        if phase == 1:
            keys.add(pygame.K_a)
        elif phase == 3:
            keys.add(pygame.K_d)
        return keys, dx, click

    if bot['unstick'] > 0:
        bot['unstick'] -= 1
        keys.add(pygame.K_s)
        dx = 25 if (frame // 120) % 2 else -25
        return keys, dx, click

    start = (int(px // TILE_SIZE), int(py // TILE_SIZE))
    goals = {(int(s.x // TILE_SIZE), int(s.y // TILE_SIZE)) for s in alive}
    path = bfs_path(start, goals)
    if path and len(path) > 1:
        tx, ty = path[min(2, len(path) - 1)]
        diff = wrap(math.atan2((ty + 0.5) * TILE_SIZE - py, (tx + 0.5) * TILE_SIZE - px) - player.angle)
        dx = int(max(-40, min(40, diff / player.sensitivity)))
        if abs(diff) < 0.7:
            keys.add(pygame.K_w)
    else:
        keys.add(pygame.K_w)
        dx = 15

    if bot['last'] is not None and abs(bot['last'][0] - px) + abs(bot['last'][1] - py) < 0.5 and pygame.K_w in keys:
        bot['stuck'] += 1
        if bot['stuck'] > 20:
            bot['stuck'] = 0
            bot['unstick'] = 25
    else:
        bot['stuck'] = 0
    bot['last'] = (px, py)
    return keys, dx, click


KEY_BITS = [(pygame.K_w, 1), (pygame.K_s, 2), (pygame.K_a, 4), (pygame.K_d, 8)]

inputs = open(os.path.join(OUT, 'inputs.txt'), 'w')
states = open(os.path.join(OUT, 'states.txt'), 'w')
crcs = open(os.path.join(OUT, 'crc.txt'), 'w')
result = 'TIMEOUT'
for frame in range(FRAMES):
    keys, dx, click = policy(frame)
    cur['keys'], cur['dx'], cur['click'] = keys, dx, click
    inputs.write(f"{sum(b for k, b in KEY_BITS if k in keys)} {dx} {int(click)}\n")

    # тело цикла из main.py
    if player.hp < 1:
        result = 'LOST'
        break
    player.movement()
    render.background(player.pos)
    walls = ray_casting(player, render.textures)
    render.vision(walls + [obj.sprite_locate(player, walls) for obj in sprites.sprite_list])
    render.weapon()
    interaction.interaction_objects()
    interaction.npc_action()
    render.mini_map()
    render.showFps(clock)
    render.showHP(player.hp)
    render.crosshair()
    try:
        interaction.check_win()
    except GameOver:
        result = 'WON'
        break

    crcs.write(f"{zlib.crc32(pygame.image.tostring(screen, 'RGB')):08x}\n")
    if frame % SAVE_EVERY == 0:
        save_png(f'frames/{frame:05d}.png')
    if frame % STATE_EVERY:
        continue
    parts = [frame, repr(player.x), repr(player.y), repr(player.angle), repr(player.hp), int(player.shot), FakeSound.plays]
    for s in sprites.sprite_list:
        parts += [repr(float(s.x)), repr(float(s.y)), repr(s.d_sprite), s.ray, int(bool(s.is_dead)),
                  int(bool(s.npc_action_trigger)), s.projection_height]
    states.write(' '.join(map(str, parts)) + '\n')

with open(os.path.join(OUT, 'result.txt'), 'w') as f:
    f.write(f'{result} {frame}\n')
print(result, frame, 'kills', sum(1 for s in sprites.sprite_list if s.is_dead), 'hp', player.hp)
