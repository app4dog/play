<template>
  <q-page class="game-page">
    <!-- Game canvas is always mounted; we overlay loading/errors -->
    <GameCanvas
      ref="gameCanvas"
      @game-ready="onGameReady"
      @game-error="onGameError"
      @score-changed="onScoreChanged"
      @audio-ready="onAudioReady"
      class="full-height"
    />

    <!-- Audio readiness indicator -->
    <div class="audio-indicator" v-show="audioReady">
      <q-chip color="green" text-color="white" icon="volume_up" dense>
        Audio Ready
      </q-chip>
    </div>
    
    <!-- Loading overlay -->
    <q-inner-loading :showing="gameLoading">
      <div class="game-loading">
        <q-spinner-puff color="primary" size="4em" />
        <p class="loading-text">🐕 Loading App4.Dog Game...</p>
      </div>
    </q-inner-loading>
    
    <!-- Error overlay -->
    <q-dialog v-model="gameErrorDialog">
      <q-card class="game-error">
        <q-card-section class="text-center">
          <q-icon name="error" color="negative" size="4em" />
          <h3>Game Loading Error</h3>
          <p>{{ gameError }}</p>
        </q-card-section>
        <q-card-actions align="right">
          <q-btn color="primary" label="Retry" @click="retryInit" />
        </q-card-actions>
      </q-card>
  </q-dialog>

  <!-- Debug Panel Dialog -->
  <q-dialog v-model="showDebugPanel">
    <q-card style="min-width: 360px; max-width: 520px">
      <q-card-section>
        <div class="text-h6">🛠️ Debug Panel</div>
        <div class="row q-gutter-sm q-mt-sm">
          <q-chip :color="audioReady ? 'green' : 'grey'" text-color="white" dense icon="volume_up">Audio {{ audioReady ? 'Ready' : 'Pending' }}</q-chip>
          <q-chip :color="eventBridgeReady ? 'indigo' : 'grey'" text-color="white" dense>Bridge {{ eventBridgeReady ? 'Ready' : 'Pending' }}</q-chip>
          <q-chip :color="nativeAudioReady ? 'purple' : 'grey'" text-color="white" dense>Native {{ nativeAudioReady ? 'Ready' : 'Pending' }}</q-chip>
          <q-chip :color="bgmGloballyDisabled ? 'red' : 'grey'" text-color="white" dense>Global BGM {{ bgmGloballyDisabled ? 'Disabled' : 'Enabled' }}</q-chip>
        </div>
      </q-card-section>

      <q-separator inset />

      <q-card-section class="q-gutter-sm">
        <div class="text-subtitle2">Audio Tests</div>
        <q-btn color="positive" label="Play Test Sound (HTMLAudio)" size="sm" @click="playTestSound" class="full-width" />
        <q-btn color="orange" label="🎵 Test Bevy Audio Bridge" size="sm" @click="testBevyAudio" :disable="!eventBridgeReady" class="full-width" />
        <q-btn color="orange-7" label="🎵 Play Provided MP3 (bridge)" size="sm" @click="testBevyAudioProvided" :disable="!eventBridgeReady" class="full-width" />
        <q-btn color="purple" label="🎶 Test Native Audio (b00t)" size="sm" @click="testNativeAudio" :disable="!nativeAudioReady" class="full-width" />
        <q-btn color="purple-7" label="🎶 Play Provided MP3 (native)" size="sm" @click="testNativeAudioProvided" :disable="!nativeAudioReady" class="full-width" />
        <div class="row q-gutter-sm">
          <q-btn color="green" label="🚪 Enter Sound" size="sm" @click="playEnterSound" :disable="!nativeAudioReady" class="col" />
          <q-btn color="red" label="🚪 Exit Sound" size="sm" @click="playExitSound" :disable="!nativeAudioReady" class="col" />
        </div>
      </q-card-section>

      <q-separator inset />

      <q-card-section>
        <div class="text-subtitle2 q-mb-sm">Audio Settings</div>
        <div class="row items-center q-gutter-sm">
          <q-toggle v-model="settings.music_enabled" label="Music" color="green" />
          <div class="col-auto text-caption">BGM</div>
          <q-slider class="col" v-model.number="settings.bgm_volume" :min="0" :max="1" :step="0.01" color="green" />
          <div class="col-auto text-caption">SFX</div>
          <q-slider class="col" v-model.number="settings.sfx_volume" :min="0" :max="1" :step="0.01" color="orange" />
        </div>
      </q-card-section>

      <q-card-actions align="right">
        <q-btn flat label="Close" color="primary" v-close-popup />
      </q-card-actions>
    </q-card>
  </q-dialog>
    
    <!-- Game menu overlay -->
    <q-dialog v-model="showMenu">
      <q-card class="game-menu font-hero">
        <q-card-section class="text-center">
          <div class="text-h4">🐕 App4.Dog Game</div>
          <div class="text-subtitle2">Interactive Pet Training</div>
        </q-card-section>
        
        <q-card-section>
          <div class="menu-stats">
            <div class="stat-item">
              <q-icon name="stars" color="amber" />
              <span>High Score: {{ highScore }}</span>
            </div>
            <div class="stat-item">
              <q-icon name="trending_up" color="positive" />
              <span>Current Level: {{ currentLevel }}</span>
            </div>
          </div>
        </q-card-section>
        
        <q-card-section class="menu-buttons">
          <q-btn
            color="primary"
            label="Start Game"
            size="lg"
            @click="startGame"
            class="full-width q-mb-sm"
          />
          <q-btn
            color="secondary"
            label="Select Critter"
            size="md"
            @click="showCritterSelection = true"
            class="full-width q-mb-sm"
          />
          <q-btn
            color="info"
            label="Training Mode"
            size="md"
            @click="startTrainingMode"
            class="full-width q-mb-sm"
          />
          <q-btn
            color="accent"
            label="Settings"
            size="md"
            @click="showSettings = true"
            class="full-width"
          />
          <q-btn
            color="grey-7"
            label="🛠️ Debug Panel"
            size="md"
            @click="showDebugPanel = true"
            class="full-width q-mt-sm"
          />
        </q-card-section>
      </q-card>
    </q-dialog>
    
    <!-- Critter selection dialog -->
    <q-dialog v-model="showCritterSelection">
      <CritterSelection
        @critter-selected="onCritterSelected"
        @close="showCritterSelection = false"
      />
    </q-dialog>
    
    <!-- Settings dialog -->
    <q-dialog v-model="showSettings">
      <GameSettings
        @settings-changed="onSettingsChanged"
        @close="showSettings = false"
      />
    </q-dialog>
  </q-page>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useQuasar } from 'quasar'
import GameCanvas from '../components/GameCanvas.vue'
import { useBevyEventBridge } from '../composables/useBevyEventBridge'
import { useNativeAudio } from '../composables/useNativeAudio'
import { useSettings } from '../composables/useSettings'
// Type for methods exposed by GameCanvas via defineExpose
type GameCanvasExposed = {
  pauseGame: () => void
  resumeGame: () => void
  resetGame: () => void
  initializeAudioContext: () => Promise<void>
  startBackgroundMusic?: (url?: string) => Promise<boolean>
  stopBackgroundMusic?: () => void
  setBackgroundMusicVolume?: (v: number) => void
  getGameEngine: () => GameEngine | null
  loadCritterById: (id: string) => void
  getCritterInfo: () => { id: number; name: string; species: string; happiness: number; energy: number } | null
}

// Type for WASM GameEngine interface
interface GameEngine {
  play_audio_via_bridge?: (soundId: string, volume: number) => string
  play_audio_native?: (soundId: string, volume: number) => string
  play_enter_sound?: () => string
  play_exit_sound?: () => string
  handle_interaction?: (type: string, x: number, y: number, dirX: number, dirY: number) => void
  load_critter?: (id: number, name: string, species: string) => void
  load_critter_by_id?: (id: string) => void
}
import CritterSelection from '../components/CritterSelection.vue'
import GameSettings from '../components/GameSettings.vue'

const $q = useQuasar()

// Game state
const gameLoading = ref(true)
const gameError = ref<string | null>(null)
const gameErrorDialog = ref(false)
const showMenu = ref(true)
const showCritterSelection = ref(false)
const showSettings = ref(false)
const showDebugPanel = ref(false)

// Game stats
const highScore = ref(0)
const currentLevel = ref(1)
const currentScore = ref(0)

// Game canvas reference
const gameCanvas = ref<GameCanvasExposed | null>(null)
const audioReady = ref(false)
const bgmGloballyDisabled = ref(window.__A4D_DISABLE_BGM__ === true)

// Event bridge for Bevy <-> TypeScript communication
const { isReady: eventBridgeReady } = useBevyEventBridge()

// Native audio handler for b00t AudioPlugin
const { isReady: nativeAudioReady } = useNativeAudio()
const { settings, sendToBevy: sendSettingsToBevy } = useSettings()

// Initialize game on mount
onMounted(() => {
  // TODO: Temporarily disable background music globally
  window.__A4D_DISABLE_BGM__ = true
  bgmGloballyDisabled.value = true  // Make reactive ref aware of the change
  console.log('🎼 Background music disabled globally via __A4D_DISABLE_BGM__')
  
  initializeGame()
  loadGameStats()
})

const initializeGame = () => {
  gameLoading.value = true
  gameError.value = null
  
  // Game will be initialized by GameCanvas component
}

const onGameReady = () => {
  gameLoading.value = false
  console.log('🎮 Game is ready to play!')
  // Sync current settings to engine
  try { sendSettingsToBevy() } catch (e) { console.warn('Failed to push settings to engine', e) }
}

const onGameError = (error: string) => {
  gameLoading.value = false
  gameError.value = error
  gameErrorDialog.value = true
  
  $q.notify({
    type: 'negative',
    message: 'Game failed to load',
    caption: error,
    position: 'top'
  })
}

const onScoreChanged = (score: number) => {
  currentScore.value = score
  
  // Update high score if needed
  if (score > highScore.value) {
    highScore.value = score
    saveGameStats()
    
    $q.notify({
      type: 'positive',
      message: 'New High Score! 🏆',
      position: 'top'
    })
  }
}

const onAudioReady = () => {
  audioReady.value = true
  $q.notify({ type: 'positive', message: '🔊 Audio ready', position: 'top', timeout: 1200 })
  // NOTE: Removed automatic BGM start to prevent race condition with startGame()
  // BGM will be started explicitly in startGame() when user clicks Start Game
  if (bgmGloballyDisabled.value) {
    $q.notify({ type: 'warning', message: '🎼 BGM disabled globally', position: 'top', timeout: 1500 })
  }
}

const startGame = async () => {
  // Initialize AudioContext on first user interaction
  await gameCanvas.value?.initializeAudioContext()
  audioReady.value = true
  showMenu.value = false
  gameCanvas.value?.resumeGame()
  // Start minimal background music (web)
  try {
    if (!bgmGloballyDisabled.value && settings.music_enabled) {
      const started = await gameCanvas.value?.startBackgroundMusic?.()
      if (started) {
        gameCanvas.value?.setBackgroundMusicVolume?.(settings.bgm_volume)
        $q.notify({ type: 'info', message: '🎼 Background music started', position: 'top', timeout: 1500 })
      }
    } else if (bgmGloballyDisabled.value) {
      $q.notify({ type: 'warning', message: '🎼 BGM disabled globally', position: 'top', timeout: 1500 })
    }
  } catch (e) {
    console.warn('BGM start failed', e)
  }
  
  $q.notify({
    type: 'info',
    message: '🐾 Let your pet play! Tap the screen to interact with critters.',
    position: 'top',
    timeout: 3000
  })
}

const retryInit = () => {
  gameErrorDialog.value = false
  initializeGame()
}

// Simple test sound playback using a real .mp3/.ogg asset path
const playTestSound = async () => {
  // Initialize AudioContext on first user interaction
  await gameCanvas.value?.initializeAudioContext()
  const base = import.meta.env.BASE_URL
  const candidates = [
    `${base}assets/audio/positive/yipee.mp3`,
    `${base}assets/audio/positive/yipee.ogg`,
    // Public sample MP3s known to work cross-origin
    'https://interactive-examples.mdn.mozilla.net/media/cc0-audio/t-rex-roar.mp3',
    'https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3',
  ]

  let lastError: unknown = null
  for (const url of candidates) {
    try {
      const audio = new Audio()
      audio.preload = 'auto'
      audio.crossOrigin = 'anonymous'
      audio.src = url
      await audio.play()
      $q.notify({ type: 'positive', message: `🔊 Playing: ${url}`, position: 'top' })
      return
    } catch (err) {
      lastError = err
      // try next candidate
    }
  }
  const describeError = (err: unknown): string => {
    if (err instanceof Error) return `${err.name}: ${err.message}`
    if (typeof err === 'string') return err
    try { return JSON.stringify(err) } catch { /* ignore */ }
    return 'Unknown error'
  }
  console.error('Audio play failed for all candidates', { candidates, lastError })
  $q.notify({ type: 'negative', message: '❌ Failed to play any test sound', caption: describeError(lastError), position: 'top' })
}

const testBevyAudio = async () => {
  // Initialize AudioContext on first user interaction
  await gameCanvas.value?.initializeAudioContext()
  if (!gameCanvas.value) {
    $q.notify({ type: 'warning', message: '❌ Game engine not ready', position: 'top' })
    return
  }

  try {
    // Call the WASM function to trigger audio via the event bridge
    const gameEngine = gameCanvas.value.getGameEngine()
    if (gameEngine && typeof gameEngine.play_audio_via_bridge === 'function') {
      const requestId = gameEngine.play_audio_via_bridge('yipee', 0.8)
      $q.notify({ 
        type: 'info', 
        message: `🎵 Audio request sent via Bevy bridge (${requestId})`, 
        position: 'top',
        timeout: 2000
      })
    } else {
      $q.notify({ type: 'warning', message: '❌ Bevy audio bridge not available', position: 'top' })
    }
  } catch (error) {
    console.error('Failed to test Bevy audio:', error)
    $q.notify({ type: 'negative', message: '❌ Failed to test Bevy audio', position: 'top' })
  }
}

const providedFile = 'assets/audio/1061796062_612948776_1752414215.mp3'

const testBevyAudioProvided = async () => {
  await gameCanvas.value?.initializeAudioContext()
  if (!gameCanvas.value) {
    $q.notify({ type: 'warning', message: '❌ Game engine not ready', position: 'top' })
    return
  }
  try {
    const gameEngine = gameCanvas.value.getGameEngine()
    if (gameEngine && typeof gameEngine.play_audio_via_bridge === 'function') {
      const requestId = gameEngine.play_audio_via_bridge(providedFile, 0.8)
      $q.notify({ type: 'info', message: `🎵 Bridge: playing ${providedFile} (${requestId})`, position: 'top', timeout: 2000 })
    }
  } catch (e) {
    console.error('Failed to test provided bridge audio:', e)
    $q.notify({ type: 'negative', message: '❌ Bridge audio failed', position: 'top' })
  }
}

const testNativeAudio = async () => {
  // Initialize AudioContext on first user interaction
  await gameCanvas.value?.initializeAudioContext()
  if (!gameCanvas.value) {
    $q.notify({ type: 'warning', message: '❌ Game engine not ready', position: 'top' })
    return
  }

  try {
    // Call the WASM function to trigger native audio via AudioPlugin
    const gameEngine = gameCanvas.value.getGameEngine()
    if (gameEngine && typeof gameEngine.play_audio_native === 'function') {
      const requestId = gameEngine.play_audio_native('yipee', 0.8)
      $q.notify({ 
        type: 'info', 
        message: `🎶 Native audio request sent (${requestId})`, 
        position: 'top',
        timeout: 2000
      })
    } else {
      $q.notify({ type: 'warning', message: '❌ Native audio not available', position: 'top' })
    }
  } catch (error) {
    console.error('Failed to test native audio:', error)
    $q.notify({ type: 'negative', message: '❌ Failed to test native audio', position: 'top' })
  }
}

const testNativeAudioProvided = async () => {
  await gameCanvas.value?.initializeAudioContext()
  if (!gameCanvas.value) {
    $q.notify({ type: 'warning', message: '❌ Game engine not ready', position: 'top' })
    return
  }
  try {
    const gameEngine = gameCanvas.value.getGameEngine()
    if (gameEngine && typeof gameEngine.play_audio_native === 'function') {
      const requestId = gameEngine.play_audio_native(providedFile, 0.8)
      $q.notify({ type: 'info', message: `🎶 Native: playing ${providedFile} (${requestId})`, position: 'top', timeout: 2000 })
    }
  } catch (e) {
    console.error('Failed to test provided native audio:', e)
    $q.notify({ type: 'negative', message: '❌ Native audio failed', position: 'top' })
  }
}

const playEnterSound = async () => {
  // Initialize AudioContext on first user interaction
  await gameCanvas.value?.initializeAudioContext()
  if (!gameCanvas.value) {
    $q.notify({ type: 'warning', message: '❌ Game engine not ready', position: 'top' })
    return
  }

  try {
    const gameEngine = gameCanvas.value.getGameEngine()
    if (gameEngine && typeof gameEngine.play_enter_sound === 'function') {
      const requestId = gameEngine.play_enter_sound()
      $q.notify({ 
        type: 'positive', 
        message: `🚪 Enter sound playing (${requestId})`, 
        position: 'top',
        timeout: 1500
      })
    } else {
      $q.notify({ type: 'warning', message: '❌ Enter sound not available', position: 'top' })
    }
  } catch (error) {
    console.error('Failed to play enter sound:', error)
    $q.notify({ type: 'negative', message: '❌ Failed to play enter sound', position: 'top' })
  }
}

const playExitSound = async () => {
  // Initialize AudioContext on first user interaction
  await gameCanvas.value?.initializeAudioContext()
  if (!gameCanvas.value) {
    $q.notify({ type: 'warning', message: '❌ Game engine not ready', position: 'top' })
    return
  }

  try {
    const gameEngine = gameCanvas.value.getGameEngine()
    if (gameEngine && typeof gameEngine.play_exit_sound === 'function') {
      const requestId = gameEngine.play_exit_sound()
      $q.notify({ 
        type: 'positive', 
        message: `🚪 Exit sound playing (${requestId})`, 
        position: 'top',
        timeout: 1500
      })
    } else {
      $q.notify({ type: 'warning', message: '❌ Exit sound not available', position: 'top' })
    }
  } catch (error) {
    console.error('Failed to play exit sound:', error)
    $q.notify({ type: 'negative', message: '❌ Failed to play exit sound', position: 'top' })
  }
}

const startTrainingMode = () => {
  showMenu.value = false
  // Future: start vocabulary training mode
  
  $q.notify({
    type: 'info',
    message: '📚 Training mode coming soon!',
    position: 'top'
  })
}

const onCritterSelected = (critter: { id: string; name: string; species: string }) => {
  showCritterSelection.value = false
  
  $q.notify({
    type: 'positive',
    message: `${critter.name} selected! 🎉`,
    position: 'top'
  })
  
  // Communicate selected critter to WASM game engine
  const gameCanvasComponent = gameCanvas.value
  if (gameCanvasComponent?.loadCritterById) {
    console.log(`🐶 Loading critter in game engine by id: ${critter.id}`)
    gameCanvasComponent.loadCritterById(critter.id)
  } else {
    console.warn('⚠️ Game engine not ready for critter loading')
  }
}

const onSettingsChanged = (settings: unknown) => {
  showSettings.value = false
  void settings
  
  $q.notify({
    type: 'positive',
    message: 'Settings saved! ⚙️',
    position: 'top'
  })
  
  // Future: apply settings to game engine
}

// React to settings changes and apply to audio
watch(() => settings.bgm_volume, (v) => {
  gameCanvas.value?.setBackgroundMusicVolume?.(v)
})
watch(() => settings.music_enabled, (enabled) => {
  if (!audioReady.value) return
  if (enabled && !bgmGloballyDisabled.value) {
    void gameCanvas.value?.startBackgroundMusic?.()
    gameCanvas.value?.setBackgroundMusicVolume?.(settings.bgm_volume)
  } else if (!enabled) {
    gameCanvas.value?.stopBackgroundMusic?.()
  } else if (bgmGloballyDisabled.value) {
    $q.notify({ type: 'warning', message: '🎼 BGM disabled globally', position: 'top', timeout: 1500 })
  }
})

// Local storage for game stats
const loadGameStats = () => {
  try {
    const saved = localStorage.getItem('app4dog-game-stats')
    if (saved) {
      const stats = JSON.parse(saved)
      highScore.value = stats.highScore || 0
      currentLevel.value = stats.currentLevel || 1
    }
  } catch (error) {
    console.warn('Failed to load game stats:', error)
  }
}

const saveGameStats = () => {
  try {
    const stats = {
      highScore: highScore.value,
      currentLevel: currentLevel.value,
      lastPlayed: new Date().toISOString()
    }
    localStorage.setItem('app4dog-game-stats', JSON.stringify(stats))
  } catch (error) {
    console.warn('Failed to save game stats:', error)
  }
}

// Handle device back button (for mobile)
onMounted(() => {
  document.addEventListener('backbutton', () => {
    if (!showMenu.value) {
      gameCanvas.value?.pauseGame()
      showMenu.value = true
    }
  })
})
</script>

<style scoped lang="scss">
.game-page {
  padding: 0;
  height: 100vh;
  overflow: hidden;
  background: url('/assets/logo/main-menu-splash-v1.png') center center / cover no-repeat fixed;
}

.game-loading,
.game-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  text-align: center;
  padding: 2rem;
}

.loading-text {
  margin-top: 1rem;
  font-size: 1.2rem;
  color: $primary;
}

.game-menu {
  min-width: 320px;
  max-width: 400px;
}

.menu-stats {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 1rem;
}

.menu-buttons {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.full-height {
  height: 100vh;
}

.audio-indicator {
  position: fixed;
  top: 8px;
  left: 8px;
  z-index: 1000;
}
</style>
