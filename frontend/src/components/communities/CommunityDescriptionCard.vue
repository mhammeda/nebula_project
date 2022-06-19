<template>
    <v-container>
        <v-row class="my-2">
            <v-card
                class="mx-auto white--text"
                max-width="70%"
                min-width="70%"
                color="secondary"
                shaped
            >
                <v-card-title>
                    About
                </v-card-title>
                <v-divider dark></v-divider>
                <v-card-text>
                    <div class="white--text">
                        {{ communityInfo.description }}
                    </div>
                </v-card-text>
            </v-card>
        </v-row>
        <v-row class="my-2">
            <v-card
                class="mx-auto"
                max-width="70%"
                min-width="70%"
                color="secondary"
                shaped
            >
                <v-card-text class="white--text">
                    <span
                        ><v-icon color="accent" class="mr-5"
                            >mdi-desktop-classic</v-icon
                        >HOST: {{ communityInfo.host }}</span
                    >
                </v-card-text>
            </v-card>
        </v-row>
        <v-row class="my-2">
            <v-card
                class="mx-auto"
                max-width="70%"
                min-width="70%"
                color="secondary"
                shaped
            >
                <v-card-text class="white--text">
                    <span
                        ><v-icon color="accent" class="mr-5"
                            >mdi-card-account-details-outline</v-icon
                        >ID: {{ communityInfo.id }}</span
                    >
                </v-card-text>
            </v-card>
        </v-row>
        <v-row class="my-2">
            <v-card
                class="mx-auto"
                max-width="70%"
                min-width="70%"
                color="secondary"
                shaped
            >
                <v-card-text class="white--text">
                    <span
                        ><v-icon color="accent" class="mr-5"
                            >mdi-clock-time-four-outline</v-icon
                        >Created: {{ getCreatedDate() }}</span
                    >
                </v-card-text>
            </v-card>
        </v-row>
        <v-row class="my-2">
            <v-card
                class="mx-auto"
                max-width="70%"
                min-width="70%"
                color="secondary"
                shaped
            >
                <v-card-title class="white--text">
                    Moderators
                </v-card-title>
                <v-divider dark></v-divider>
                <v-card-text>
                    <v-list color="secondary">
                        <v-list-item
                            v-for="moderator in communityInfo.moderators"
                            :key="moderator.username"
                        >
                            <v-list-item-avatar>
                                <img src="../../assets/defaultUserIcon.jpg" />
                            </v-list-item-avatar>
                            <v-list-item-content>
                                <v-list-item-title class="white--text text-h6"
                                    ><button
                                        @click="
                                            redirrectUser(moderator.username)
                                        "
                                    >
                                        {{ moderator.username }}
                                    </button></v-list-item-title
                                >
                            </v-list-item-content>
                        </v-list-item>
                    </v-list>
                </v-card-text>
            </v-card>
        </v-row>
    </v-container>
</template>
<script>
export default {
    props: ['communityInfo'],
    mounted() {
        console.log(this.communityInfo)
    },

    data() {
        return {
            url: '/internal/users',
        }
    },

    methods: {
        getCreatedDate() {
            const d = new Date(this.communityInfo.created)
            return d.toUTCString()
        },

        redirrectUser(user) {
            const url = `/user/${user}`

            this.$router.push(url)
        },
    },
}
</script>
